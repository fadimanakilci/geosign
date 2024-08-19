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
use std::str::FromStr;
use std::sync::Mutex;
use chrono::{DateTime, FixedOffset, Utc};
use qdrant_client::{Qdrant, QdrantError};
use qdrant_client::qdrant::{CreateCollectionBuilder, Distance, VectorParamsBuilder, PointStruct, UpsertPointsBuilder, SearchPointsBuilder, ListCollectionsResponse, Condition, Filter, SearchParamsBuilder, QueryResponse, Vector, Vectors, Value, PointId};

use lazy_static::lazy_static;
use serde::{Serialize, Serializer};
use serde_json::map::Values;
use tokio_postgres::{Client, Error, NoTls, Row};
use tokio_postgres::types::Date;

lazy_static! {
    static ref client: Qdrant = {
        // The Rust client uses Qdrant's GRPC interface
        let mut _client = Qdrant::from_url("http://localhost:6334").build().unwrap();
        _client
    };

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

#[tokio::main]
async fn main() -> Result<(), QdrantError> {
    // Config UTC+3
    set_utc();

    // Connecting PostgreSql
    set_connect_db().await.unwrap();

    // Create the vector database
    create_collection().await?;

    // Running the query
    query().await?;

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
        tokio_postgres::connect("postgresql://postgres:mydevpasswd000@78.186.223.18:5432/py_algida0", NoTls).await?;

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
    let rows = client_postgresql
        .query("SELECT location_data_id, index, device_id, vehicle_id,\
         user_id, m_code, mt_id, type::text, device_time::text, server_time::text, locale,\
          coordinate, ignition_on, speed::text, distance::text, total_distance::text,\
           engine_hours FROM public.location_data \
           ORDER BY location_data_id DESC LIMIT 100", &[])
        .await?;

    // Global ROWS değişkenine atama yapıyoruz
    let mut global_rows = data_rows.lock().unwrap();
    *global_rows = Some(rows);

    Ok(())
}

async fn create_collection() -> Result<(), QdrantError> {
    let collections_list: Option<ListCollectionsResponse> = Some(client.list_collections().await?);

    println!("Test: {}", collections_list.as_ref().map_or(false, |list| list.collections.len() >= 1));

    if collections_list.is_none() || collections_list.as_ref().map_or(false, |list| list.collections.len() >= 1) {
        client
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
    let points = vec![
        PointStruct::new(1, vec![0.05, 0.61, 0.76, 0.74], [("city", "Berlin".into())]),
        PointStruct::new(2, vec![0.19, 0.81, 0.75, 0.11], [("city", "London".into())]),
        PointStruct::new(3, vec![0.36, 0.55, 0.47, 0.94], [("city", "Moscow".into())]),
        // ..truncated
    ];

    let response = client
        .upsert_points(UpsertPointsBuilder::new("test_collection", points).wait(true))
        .await?;

    dbg!(response);

    Ok(())
}

async fn query() -> Result<(), QdrantError> {
    let search_result = client
        .search_points(
            SearchPointsBuilder::new("test_collection", [0.2, 0.1, 0.9, 0.7], 3).with_payload(true),
        )
        .await?;

    dbg!(search_result);

    Ok(())
}
