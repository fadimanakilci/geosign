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
use qdrant_client::qdrant::{CreateCollectionBuilder, Distance, VectorParamsBuilder, PointStruct, UpsertPointsBuilder};

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
    create_collection().await?;

    Ok(())
}

async fn create_collection() -> Result<(), QdrantError> {
    client
        .create_collection(
            CreateCollectionBuilder::new("test_collection")
                .vectors_config(VectorParamsBuilder::new(4, Distance::Dot)),
        )
        .await?;

    Ok(())
}
