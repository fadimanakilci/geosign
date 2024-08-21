/*
 * DO NOT ALTER OR REMOVE COPYRIGHT NOTICES OR THIS FILE HEADER.
 *
 * Copyright © August 2024 Fadimana Kilci - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited
 * Proprietary and confidential
 *
 * Created by Fadimana Kilci  <fadimekilci07@gmail.com>, August 2024
 */
use std::arch::aarch64::{int64x1_t, int8x16x2_t, int8x8_t};
use std::collections::HashMap;
use std::ptr::null;
use std::str::FromStr;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use actix_files as fs;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use chrono::{DateTime, FixedOffset, Utc};
use qdrant_client::{Qdrant, QdrantError};
use qdrant_client::qdrant::{CreateCollectionBuilder, Distance, VectorParamsBuilder, PointStruct, UpsertPointsBuilder, SearchPointsBuilder, ListCollectionsResponse, Condition, Filter, SearchParamsBuilder, QueryResponse, Vector, Vectors, Value, PointId, GeoRadius, GeoPoint, SearchResponse, Struct};

use lazy_static::lazy_static;
use log::info;
use qdrant_client::qdrant::value::Kind;
use qdrant_client::qdrant::value::Kind::StructValue;
use serde::{Serialize, Serializer};
use serde_json::json;
use serde_json::map::Values;
use sysinfo::System;
use tokio::time::timeout;
use tokio_postgres::{Client, Error, NoTls, Row};
use tokio_postgres::types::Date;
use tonic::transport::Channel;
use uuid::Uuid;

lazy_static! {
    static ref CLIENT: Qdrant = {
        // The Rust client uses Qdrant's GRPC interface
        let mut _client = Qdrant::from_url("http://localhost:6334").timeout(Duration::from_secs(3200)).build().unwrap();
        _client
    };

    // static ref CLIENT: Qdrant = {
    //     // tonic::transport::Channel üzerinden bir yapılandırma yaparak timeout ayarını ekleyin
    //     let channel = Channel::from_static("http://localhost:6334")
    //         .connect_timeout(Duration::from_secs(30)) // Timeout süresini 10 saniye olarak ayarladık
    //         .timeout(Duration::from_secs(120)); // Genel timeout süresi (request başına)
    //
    //     // Qdrant istemcisini bu yapılandırılmış channel ile oluşturun
    //     Qdrant::with_channel(channel)
    // };

    static ref data_rows: Mutex<Option<Vec<Row>>> = Mutex::new(None);
}

#[derive(Serialize)]
struct QdrantPoint {
    vector: [f64; 2],
    payload: Payload,
}

#[derive(Serialize)]
struct Payload {
    id: i64,
    index: i32,
    device_id: String,
    vehicle_id: String,
    user_id: String,
    m_code: String,
    mt_id: i64,
    con_type: String,
    #[serde(serialize_with = "serialize_date_time")]
    device_time: DateTime<FixedOffset>,
    #[serde(serialize_with = "serialize_date_time")]
    server_time: DateTime<FixedOffset>,
    locale: String,
    coordinate: String,
    ignition_on: bool,
    speed: i32,
    distance: i32,
    total_distance: i32,
    engine_hours: i32,
}

// #[tokio::main]
#[actix_web::main]
async fn main() -> Result<(), QdrantError> {
    env_logger::init();

    // Config UTC+3
    set_utc();

    // Connecting PostgreSql
    set_connect_db().await.unwrap();

    // Create the vector database
    create_collection().await?;

    // Running the test_collection query
    // query().await?;

    // Running the test_collection query
    let response = query_geo().await?;

    let response = query_special_for_cagri().await?;

    send_to_map(response).await?;

    Ok(())
}

fn serialize_date_time<S>(datetime: &DateTime<FixedOffset>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&datetime.to_rfc3339())
}

fn set_utc() {
    // UTC+3 saat dilimi
    let offset = FixedOffset::east(3 * 3600); // 3 saat doğu (3600 saniye = 1 saat)

    // Örnek olarak şu anki zamanı UTC+3 olarak alıyoruz
    let now: DateTime<FixedOffset> = chrono::Utc::now().with_timezone(&offset);

    println!("Şu anki UTC+3 zamanı: {}", now);
}

async fn set_connect_db() -> Result<(), Error> {
    // PostgreSQL bağlantısı
    let (client_postgresql, connection) =
        tokio_postgres::connect("postgresql://name:password@host:port/table_name", NoTls).await?;

    // Bağlantıyı yönetmek için ayrı bir task
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    fetch_data(&client_postgresql).await?;

    Ok(())
}

async fn fetch_data(client_postgresql: &Client) -> Result<(), Error> {
    // let rows = client_postgresql
    //     .query("SELECT location_data_id, index, device_id, vehicle_id,\
    //      user_id, m_code, mt_id, type::text, device_time::text, server_time::text, locale,\
    //       coordinate, ignition_on, speed::text, distance::text, total_distance::text,\
    //        engine_hours FROM public.location_data \
    //        ORDER BY location_data_id DESC LIMIT 100", &[])
    //     .await?;

    let rows = client_postgresql
        .query("SELECT location_data_id, coordinate \
         FROM public.location_data \
         ORDER BY location_data_id \
         DESC LIMIT 1000000", &[])
        .await?;

    // Global ROWS değişkenine atama yapıyoruz
    let mut global_rows = data_rows.lock().unwrap();
    *global_rows = Some(rows);

    Ok(())
}

async fn create_collection() -> Result<(), QdrantError> {
    let collections_list: Option<ListCollectionsResponse> = Some(CLIENT.list_collections().await?);

    println!("Test: {}", collections_list.as_ref().map_or(false, |list| list.collections.len() >= 1));

    if collections_list.is_none() || collections_list.as_ref().map_or(false, |list| list.collections.len() >= 1) {
        CLIENT
            .create_collection(
                CreateCollectionBuilder::new("geo_collection")
                    .vectors_config(VectorParamsBuilder::new(2, Distance::Dot)),
            )
            .await?;

        add_vectors().await?;
    } else {
        dbg!(collections_list);
    }

    Ok(())
}

async fn add_vectors() -> Result<(), QdrantError> {
    // let points = vec![
    //     PointStruct::new(1, vec![0.05, 0.61, 0.76, 0.74], [("city", "Berlin".into())]),
    //     PointStruct::new(2, vec![0.19, 0.81, 0.75, 0.11], [("city", "London".into())]),
    //     PointStruct::new(3, vec![0.36, 0.55, 0.47, 0.94], [("city", "Moscow".into())]),
    //     // ..truncated
    // ];

    let global_rows = data_rows.lock().unwrap();
    let mut points: Vec<PointStruct> = Vec::new();
    let mut payload_map: HashMap<String, Value> = HashMap::new();

    let mut counter: i32 = 1;

    if let Some(ref rows) = *global_rows {
        for row in rows.iter() {

            println!("Control counter: {}", counter);
            counter += 1;

            let mut latitude: f32 = 0.0;
            let mut longitude: f32 = 0.0;

            match parse_coordinate(row.get(11)) {
                Ok((lat, long)) => {
                    latitude = lat;
                    longitude = long;
                },
                Err(e) => {
                    eprintln!("Error parsing coordinate: {}", e);
                }
            }

            // GeoPoint formatında JSON oluşturun
            let geo_point = json!({
                "lat": latitude,
                "lon": longitude
            });

            let speed_text: String = row.get(13);
            let distance: String = row.get(14);
            let total_distance: String = row.get(15);
            let numeric_value_speed = f64::from_str(&speed_text).unwrap_or_default();
            let numeric_value_distance = f64::from_str(&distance).unwrap_or_default();
            let numeric_value_total = f64::from_str(&total_distance).unwrap_or_default();

            payload_map.insert("id".to_string(), Value::from(row.get::<_, i32>(0) as i64));
            payload_map.insert("index".to_string(), Value::from(row.get::<_, i32>(1) as i64));
            payload_map.insert("device_id".to_string(), Value::from(row.get::<_, String>(2)));
            payload_map.insert("vehicle_id".to_string(), Value::from(row.get::<_, String>(3)));
            payload_map.insert("user_id".to_string(), Value::from(row.get::<_, String>(4)));
            payload_map.insert("m_code".to_string(), Value::from(row.get::<_, String>(5)));
            payload_map.insert("mt_id".to_string(), Value::from(row.get::<_, String>(6)));
            payload_map.insert("con_type".to_string(), Value::from(row.get::<_, String>(7)));
            payload_map.insert("device_time".to_string(), Value::from(row.get::<_, String>(8)));
            payload_map.insert("server_time".to_string(), Value::from(row.get::<_, String>(9)));
            payload_map.insert("locale".to_string(), Value::from(row.get::<_, String>(10)));
            // payload_map.insert("coordinate".to_string(), Value::from(row.get::<_, String>(11)));
            // payload_map.insert("coordinate".to_string(), Value::from(geo_point.as_object().unwrap().clone()));
            payload_map.insert("coordinate".to_string(), Value::from(geo_point));
            payload_map.insert("ignition_on".to_string(), Value::from(row.get::<_, bool>(12)));
            payload_map.insert("speed".to_string(), Value::from(numeric_value_speed));
            payload_map.insert("distance".to_string(), Value::from(numeric_value_distance));
            payload_map.insert("total_distance".to_string(), Value::from(numeric_value_total));
            payload_map.insert("engine_hours".to_string(), Value::from(row.get::<_, i32>(16) as i64));

            let id: String = row.get::<usize, i32>(0).to_string();
            let uuid = Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::new_v4());
            let point_id: PointId = PointId::from(uuid.to_string());

            let point = PointStruct::new(
                point_id,
                vec![latitude, longitude],
                payload_map.clone());

            // println!("Control: {:?}", point);

            points.push(point);
        }
    }

    let response = client
        .upsert_points(UpsertPointsBuilder::new("geo_collection", points).wait(true))
    println!("Control Points: {}", points.len());

    let response = CLIENT
        // .upsert_points(UpsertPointsBuilder::new("geo_collection", points).wait(true))
        .upsert_points(UpsertPointsBuilder::new("cagri_abi_icin_xd", points).wait(true))
        .await?;

    // let response = timeout(Duration::from_secs(12000), async {
    //     CLIENT
    //         .upsert_points(UpsertPointsBuilder::new("cagri_abi_icin_xd", points).wait(true))
    //         .await
    // }).await;

    // let chunk_size = 1_000;
    // for chunk in points.chunks(chunk_size) {
    //     dbg!("Control counter: {}", counter);
    //     counter += 1;
    //
    //     let response = CLIENT
    //         .upsert_points(UpsertPointsBuilder::new("cagri_abi_icin_xd", chunk.to_vec()).wait(true))
    //         .await?;
    // }


    // // Veriyi Qdrant'a gönder
    // let response = http_client
    //     .post(qdrant_url)
    //     .json(&point_data)
    //     .send()
    //     .await;

    Ok(())
}

async fn query() -> Result<(), QdrantError> {
    let search_result = CLIENT
        .search_points(
            SearchPointsBuilder::new(
                "test_collection",
                [0.2, 0.1, 0.9, 0.7],
                3)
                .filter(Filter::all([Condition::matches(
                    "city",
                    "London".to_string(),
                )]))
                .with_payload(true)
                .params(SearchParamsBuilder::default().exact(true)),
        )
        .await?;

    dbg!(search_result);

    Ok(())
}

// 37.850897, 32.457679
// 37.864218, 32.455185
async fn query_geo() -> Result<SearchResponse, QdrantError> {
    let filter = Filter::all([
        Condition::geo_radius(
            "coordinate",
            GeoRadius {
                center: Some(GeoPoint {
                    lon: 32.455185,
                    lat: 37.864218
                }),
                // metre
                radius: 10000.0,
            },
        ),
        // Condition::matches("city", "London".to_string()),
    ]);

    let search_result = CLIENT
        .search_points(
            SearchPointsBuilder::new(
                "geo_collection",
                [37.864218, 32.455185],
                1000000)
                .filter(filter)
                .with_payload(true)
                .params(SearchParamsBuilder::default().exact(true)),
        )
        .await?;

    dbg!(&search_result);

    Ok(search_result)
}

async fn send_to_map(response: SearchResponse) -> Result<(), QdrantError> {
    println!("Coordinate Len: {}", response.result.len());
    let coordinates: Vec<_> = response
        .result
        .iter()
        .filter_map(|scored_point| {
            // println!("Girdi {:?}", scored_point.payload);

            if let Some(Value {
                            kind: Some(StructValue(Struct { fields })),
                        }) = scored_point.payload.get("coordinate")
            {

                // println!("Girdi 2");

                let lat = match fields.get("lat")?.kind.as_ref()? {
                    Kind::DoubleValue(val) => *val,
                    _ => return None,
                };
                let lon = match fields.get("lon")?.kind.as_ref()? {
                    Kind::DoubleValue(val) => *val,
                    _ => return None,
                };

                Some(json!({"lat": lat, "lon": lon}))
            } else {
                None
            }
        })
        .collect();

    let data = json!({
        "coordinates": coordinates,
        "center": {"lat": 37.870112, "lon": 32.526084},
        "radius": 1000.0
    });

    info!("Starting server at http://127.0.0.1:8080/locations");

    // HttpServer::new(|| {
    //     let data = web::Data::new(json!({
    //         "coordinates": [
    //             {"lat": 40.712776, "lon": -74.005974}, // New York City
    //             {"lat": 34.052235, "lon": -118.243683}, // Los Angeles
    //         ],
    //         "radius": 1000
    //     }));
    //
    //     App::new()
    //         .app_data(data.clone()) // JSON verisini paylaşılabilir hale getiriyoruz
    //         .route("/locations", web::get().to(get_locations)) // JSON verisi döndüren rota
    //         .service(fs::Files::new("/", "./static").index_file("leaflet_map.html")) // Statik dosyalar
    // })
    //     .bind("127.0.0.1:8080")?
    //     .run()
    //     .await?;

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(data.clone())) // JSON verisini paylaşılabilir hale getiriyoruz
            .route("/locations", web::get().to(get_locations)) // Handler fonksiyonunu kullanıyoruz
            .service(fs::Files::new("/", "./static").index_file("map_radius.html"))
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await?;

    Ok(())
}

fn parse_coordinate(coordinate: String) -> Result<(f32, f32), QdrantError> {
    let parts: Vec<&str> = coordinate.split(',').collect();

    if parts.len() == 2 {
        let latitude: f32 = parts[0].trim().parse().map_err(|_| QdrantError::ConversionError("Invalid latitude".into()))?;
        let longitude: f32 = parts[1].trim().parse().map_err(|_| QdrantError::ConversionError("Invalid longitude".into()))?;

        Ok((latitude, longitude))
    } else {
        Err(QdrantError::ConversionError("Coordinate format is invalid".into()))
    }
}
