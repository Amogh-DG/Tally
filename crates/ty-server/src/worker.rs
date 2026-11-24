use std::collections::HashMap;
use std::thread;
use chrono::{DateTime, Utc};
use tokio::sync::{mpsc, oneshot};
use ty_types::{Bucket, Event, Store, StoreError};

pub enum StoreCommand {
    CreateBucket {
        bucket: Bucket,
        resp: oneshot::Sender<Result<(), StoreError>>,
    },
    GetBucket {
        id: String,
        resp: oneshot::Sender<Result<Bucket, StoreError>>,
    },
    ListBuckets {
        resp: oneshot::Sender<Result<HashMap<String, Bucket>, StoreError>>,
    },
    DeleteBucket {
        id: String,
        resp: oneshot::Sender<Result<(), StoreError>>,
    },
    InsertEvent {
        bucket_id: String,
        event: Event,
        resp: oneshot::Sender<Result<Event, StoreError>>,
    },
    GetLastEvent {
        bucket_id: String,
        resp: oneshot::Sender<Result<Option<Event>, StoreError>>,
    },
    ReplaceLastEvent {
        bucket_id: String,
        event: Event,
        resp: oneshot::Sender<Result<Event, StoreError>>,
    },
    GetEvents {
        bucket_id: String,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        limit: Option<usize>,
        resp: oneshot::Sender<Result<Vec<Event>, StoreError>>,
    },
    EventCount {
        bucket_id: String,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        resp: oneshot::Sender<Result<usize, StoreError>>,
    },
    DeleteEvent {
        bucket_id: String,
        event_id: i64,
        resp: oneshot::Sender<Result<(), StoreError>>,
    },
}

#[derive(Clone)]
pub struct StoreClient {
    tx: mpsc::Sender<StoreCommand>,
}

impl StoreClient {
    pub async fn create_bucket(&self, bucket: Bucket) -> Result<(), StoreError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::CreateBucket {
                bucket,
                resp: resp_tx,
            })
            .await
            .map_err(|_| StoreError::Internal("store worker channel closed".into()))?;
        resp_rx
            .await
            .map_err(|_| StoreError::Internal("store worker dropped response".into()))?
    }

    pub async fn get_bucket(&self, id: String) -> Result<Bucket, StoreError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::GetBucket {
                id,
                resp: resp_tx,
            })
            .await
            .map_err(|_| StoreError::Internal("store worker channel closed".into()))?;
        resp_rx
            .await
            .map_err(|_| StoreError::Internal("store worker dropped response".into()))?
    }

    pub async fn list_buckets(&self) -> Result<HashMap<String, Bucket>, StoreError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::ListBuckets { resp: resp_tx })
            .await
            .map_err(|_| StoreError::Internal("store worker channel closed".into()))?;
        resp_rx
            .await
            .map_err(|_| StoreError::Internal("store worker dropped response".into()))?
    }

    pub async fn delete_bucket(&self, id: String) -> Result<(), StoreError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::DeleteBucket {
                id,
                resp: resp_tx,
            })
            .await
            .map_err(|_| StoreError::Internal("store worker channel closed".into()))?;
        resp_rx
            .await
            .map_err(|_| StoreError::Internal("store worker dropped response".into()))?
    }

    pub async fn insert_event(&self, bucket_id: String, event: Event) -> Result<Event, StoreError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::InsertEvent {
                bucket_id,
                event,
                resp: resp_tx,
            })
            .await
            .map_err(|_| StoreError::Internal("store worker channel closed".into()))?;
        resp_rx
            .await
            .map_err(|_| StoreError::Internal("store worker dropped response".into()))?
    }

    pub async fn get_last_event(&self, bucket_id: String) -> Result<Option<Event>, StoreError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::GetLastEvent {
                bucket_id,
                resp: resp_tx,
            })
            .await
            .map_err(|_| StoreError::Internal("store worker channel closed".into()))?;
        resp_rx
            .await
            .map_err(|_| StoreError::Internal("store worker dropped response".into()))?
    }

    pub async fn replace_last_event(
        &self,
        bucket_id: String,
        event: Event,
    ) -> Result<Event, StoreError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::ReplaceLastEvent {
                bucket_id,
                event,
                resp: resp_tx,
            })
            .await
            .map_err(|_| StoreError::Internal("store worker channel closed".into()))?;
        resp_rx
            .await
            .map_err(|_| StoreError::Internal("store worker dropped response".into()))?
    }

    pub async fn get_events(
        &self,
        bucket_id: String,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Result<Vec<Event>, StoreError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::GetEvents {
                bucket_id,
                start,
                end,
                limit,
                resp: resp_tx,
            })
            .await
            .map_err(|_| StoreError::Internal("store worker channel closed".into()))?;
        resp_rx
            .await
            .map_err(|_| StoreError::Internal("store worker dropped response".into()))?
    }

    pub async fn event_count(
        &self,
        bucket_id: String,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Result<usize, StoreError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::EventCount {
                bucket_id,
                start,
                end,
                resp: resp_tx,
            })
            .await
            .map_err(|_| StoreError::Internal("store worker channel closed".into()))?;
        resp_rx
            .await
            .map_err(|_| StoreError::Internal("store worker dropped response".into()))?
    }

    pub async fn delete_event(&self, bucket_id: String, event_id: i64) -> Result<(), StoreError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(StoreCommand::DeleteEvent {
                bucket_id,
                event_id,
                resp: resp_tx,
            })
            .await
            .map_err(|_| StoreError::Internal("store worker channel closed".into()))?;
        resp_rx
            .await
            .map_err(|_| StoreError::Internal("store worker dropped response".into()))?
    }
}

pub struct StoreWorker {
    pub client: StoreClient,
    pub thread_handle: Option<thread::JoinHandle<()>>,
}

impl StoreWorker {
    pub fn spawn(store: Box<dyn Store>) -> Self {
        let (tx, mut rx) = mpsc::channel::<StoreCommand>(256);

        let thread_handle = thread::Builder::new()
            .name("store-worker".to_string())
            .spawn(move || {
                while let Some(cmd) = rx.blocking_recv() {
                    match cmd {
                        StoreCommand::CreateBucket { bucket, resp } => {
                            let res = store.create_bucket(&bucket);
                            let _ = resp.send(res);
                        }
                        StoreCommand::GetBucket { id, resp } => {
                            let res = store.get_bucket(&id);
                            let _ = resp.send(res);
                        }
                        StoreCommand::ListBuckets { resp } => {
                            let res = store.list_buckets();
                            let _ = resp.send(res);
                        }
                        StoreCommand::DeleteBucket { id, resp } => {
                            let res = store.delete_bucket(&id);
                            let _ = resp.send(res);
                        }
                        StoreCommand::InsertEvent {
                            bucket_id,
                            event,
                            resp,
                        } => {
                            let res = store.insert_event(&bucket_id, &event);
                            let _ = resp.send(res);
                        }
                        StoreCommand::GetLastEvent { bucket_id, resp } => {
                            let res = store.get_last_event(&bucket_id);
                            let _ = resp.send(res);
                        }
                        StoreCommand::ReplaceLastEvent {
                            bucket_id,
                            event,
                            resp,
                        } => {
                            let res = store.replace_last_event(&bucket_id, &event);
                            let _ = resp.send(res);
                        }
                        StoreCommand::GetEvents {
                            bucket_id,
                            start,
                            end,
                            limit,
                            resp,
                        } => {
                            let res = store.get_events(&bucket_id, start, end, limit);
                            let _ = resp.send(res);
                        }
                        StoreCommand::EventCount {
                            bucket_id,
                            start,
                            end,
                            resp,
                        } => {
                            let res = store.event_count(&bucket_id, start, end);
                            let _ = resp.send(res);
                        }
                        StoreCommand::DeleteEvent {
                            bucket_id,
                            event_id,
                            resp,
                        } => {
                            let res = store.delete_event(&bucket_id, event_id);
                            let _ = resp.send(res);
                        }
                    }
                }
            })
            .expect("failed to spawn store worker thread");

        Self {
            client: StoreClient { tx },
            thread_handle: Some(thread_handle),
        }
    }
}
