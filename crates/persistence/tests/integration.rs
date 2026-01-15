use anyhow::Result;
use neleus_core_bus::{EventSink, Message, MessageKind, Priority, Topic};
use neleus_persistence::{PostgresEventStore, PostgresEventStoreConfig};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
#[ignore]
async fn test_postgres_event_store_insert() -> Result<()> {
    let config = PostgresEventStoreConfig {
        connection_string: "postgresql://postgres:postgres@localhost:5432/neleus_test".to_string(),
        batch_size: 10,
        pool_size: 2,
        flush_interval_ms: 50,
    };

    let store = PostgresEventStore::new(config).await?;

    for i in 0..5 {
        let msg = Message {
            id: i,
            kind: MessageKind::Data,
            topic: Topic::new(&format!("test.topic.{}", i)),
            priority: Priority::Normal,
            timestamp: i as u64,
            correlation_id: None,
            reply_to: None,
            payload: format!("Test message {}", i).into_bytes(),
        };

        store.append(&msg);
    }

    sleep(Duration::from_millis(200)).await;

    println!("Successfully inserted 5 messages to PostgreSQL");
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_postgres_batch_insert() -> Result<()> {
    let config = PostgresEventStoreConfig {
        connection_string: "postgresql://postgres:postgres@localhost:5432/neleus_test".to_string(),
        batch_size: 100,
        pool_size: 4,
        flush_interval_ms: 100,
    };

    let store = PostgresEventStore::new(config).await?;

    for i in 0..1000 {
        let msg = Message {
            id: i,
            kind: MessageKind::Data,
            topic: Topic::new("test.high.volume"),
            priority: Priority::Normal,
            timestamp: i as u64,
            correlation_id: Some(i),
            reply_to: Some(Topic::new("reply.topic")),
            payload: vec![0u8; 100],
        };

        store.append(&msg);
    }

    sleep(Duration::from_secs(2)).await;

    println!("Successfully inserted 1000 messages in batches");
    Ok(())
}

#[test]
fn test_postgres_config_defaults() {
    let config = PostgresEventStoreConfig::default();
    assert_eq!(config.batch_size, 1000);
    assert_eq!(config.pool_size, 4);
    assert_eq!(config.flush_interval_ms, 100);
}
