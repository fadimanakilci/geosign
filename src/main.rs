/*
 * DO NOT ALTER OR REMOVE COPYRIGHT NOTICES OR THIS FILE HEADER.
 *
 * Copyright © August 2024 Fadimana Kilci - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited
 * Proprietary and confidential
 *
 * Created by Fadimana Kilci  <fadimekilci07@gmail.com>, August 2024
 */

use qdrant_client::{Qdrant, QdrantError};
use qdrant_client::qdrant::{CreateCollectionBuilder, Distance, VectorParamsBuilder, PointStruct, UpsertPointsBuilder, SearchPointsBuilder, ListCollectionsResponse};

use lazy_static::lazy_static;

lazy_static! {
    static ref client: Qdrant = {
        // The Rust client uses Qdrant's GRPC interface
        let mut _client = Qdrant::from_url("http://localhost:6334").build().unwrap();
        _client
    };
}

#[tokio::main]
async fn main() -> Result<(), QdrantError> {
    // Create the vector database
    create_collection().await?;

    // Running the query
    query().await?;

    Ok(())
}

async fn create_collection() -> Result<(), QdrantError> {
    let collections_list: Option<ListCollectionsResponse> = Some(client.list_collections().await?);

    if collections_list.is_none() {
        client
            .create_collection(
                CreateCollectionBuilder::new("test_collection")
                    .vectors_config(VectorParamsBuilder::new(4, Distance::Dot)),
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
