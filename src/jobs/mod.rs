pub mod data_types;

use core::fmt::Write;
use mqttrust::{Mqtt, QoS, SubscribeTopic};

use self::data_types::{JobStatus, StatusDetails};
use crate::jobs::data_types::{
    DescribeJobExecutionRequest, GetPendingJobExecutionsRequest,
    StartNextPendingJobExecutionRequest, UpdateJobExecutionRequest, MAX_CLIENT_TOKEN_LEN,
    MAX_JOB_ID_LEN, MAX_THING_NAME_LEN,
};

use bitflags::bitflags;

bitflags! {
    pub struct Topics: u64 {
        const NOTIFY = 0x0001;
        const NOTIFY_NEXT = 0x0002;
        const GET_ACCEPTED = 0x0004;
        const GET_REJECTED = 0x0008;
        const START_NEXT_ACCEPTED = 0x0010;
        const START_NEXT_REJECTED = 0x0020;
        const DESCRIBE_SUCCESS = 0x0040;
        const DESCRIBE_FAILED = 0x0080;
        const UPDATE_SUCCESS = 0x0100;
        const UPDATE_FAILED = 0x0200;
    }
}

macro_rules! topic {
    ($fmt:expr, $($args:tt)*) => {{
        let mut topic_path = heapless::String::new();
        topic_path
            .write_fmt(format_args!(
                $fmt,
                $($args)*
            ))
            .map_err(drop)?;

        topic_path
    }};
}

pub struct Jobs;

impl Jobs {
    pub fn get_pending<M: Mqtt>(mqtt: &M) -> Result<(), ()> {
        let mut topic = heapless::String::<{ MAX_THING_NAME_LEN + 21 }>::new();

        topic
            .write_fmt(format_args!("$aws/things/{}/jobs/get", mqtt.client_id()))
            .map_err(drop)?;

        let buf = &mut [0u8; MAX_CLIENT_TOKEN_LEN];
        let len =
            serde_json_core::to_slice(&GetPendingJobExecutionsRequest { client_token: None }, buf)
                .map_err(drop)?;

        mqtt.publish(topic.as_str(), &buf[..len], QoS::AtLeastOnce)
            .map_err(drop)?;

        Ok(())
    }

    pub fn start_next<M: Mqtt>(mqtt: &M) -> Result<(), ()> {
        let mut topic = heapless::String::<{ MAX_THING_NAME_LEN + 28 }>::new();

        topic
            .write_fmt(format_args!(
                "$aws/things/{}/jobs/start-next",
                mqtt.client_id()
            ))
            .map_err(drop)?;

        let buf = &mut [0u8; MAX_CLIENT_TOKEN_LEN];
        let len = serde_json_core::to_slice(
            &StartNextPendingJobExecutionRequest {
                step_timeout_in_minutes: None,
                client_token: None,
            },
            buf,
        )
        .map_err(drop)?;

        mqtt.publish(topic.as_str(), &buf[..len], QoS::AtLeastOnce)
            .map_err(drop)?;

        Ok(())
    }

    pub fn describe_next<M: Mqtt>(mqtt: &M, client_token: Option<&str>) -> Result<(), ()> {
        Self::describe(mqtt, "$next", client_token)
    }

    pub fn describe<M: Mqtt>(mqtt: &M, job_id: &str, client_token: Option<&str>) -> Result<(), ()> {
        if client_token
            .map(|c| c.len() > MAX_CLIENT_TOKEN_LEN)
            .unwrap_or(false)
        {
            return Err(());
        }

        let mut topic_path =
            heapless::String::<{ MAX_THING_NAME_LEN + MAX_JOB_ID_LEN + 22 }>::new();

        topic_path
            .write_fmt(format_args!(
                "$aws/things/{}/jobs/{}/get",
                mqtt.client_id(),
                job_id
            ))
            .map_err(drop)?;

        let buf = &mut [0u8; { MAX_CLIENT_TOKEN_LEN + 2 }];
        let len = serde_json_core::to_slice(
            &DescribeJobExecutionRequest {
                execution_number: None,
                include_job_document: None,
                client_token,
            },
            buf,
        )
        .map_err(drop)?;

        mqtt.publish(topic_path.as_str(), &buf[..len], QoS::AtLeastOnce)
            .map_err(drop)?;

        Ok(())
    }

    pub fn update<M: Mqtt>(
        mqtt: &M,
        job_id: &str,
        status: JobStatus,
        status_details: Option<&StatusDetails>,
        qos: QoS,
    ) -> Result<(), ()> {
        let buf = &mut [0u8; 512];
        let len = serde_json_core::to_slice(
            &UpdateJobExecutionRequest {
                execution_number: None,
                expected_version: None,
                include_job_document: None,
                include_job_execution_state: None,
                status,
                status_details,
                step_timeout_in_minutes: None,
                client_token: None,
            },
            buf,
        )
        .map_err(drop)?;

        // Publish the string created above
        let mut topic_path =
            heapless::String::<{ MAX_THING_NAME_LEN + MAX_JOB_ID_LEN + 25 }>::new();
        topic_path
            .write_fmt(format_args!(
                "$aws/things/{}/jobs/{}/update",
                mqtt.client_id(),
                job_id
            ))
            .map_err(drop)?;

        mqtt.publish(topic_path.as_str(), &buf[..len], qos)
            .map_err(drop)?;

        Ok(())
    }

    pub fn subscribe<M: Mqtt>(
        mqtt: &M,
        topic_mask: Topics,
        job_id: Option<&str>,
    ) -> Result<(), ()> {
        if topic_mask.intersects(
            Topics::DESCRIBE_SUCCESS
                | Topics::DESCRIBE_FAILED
                | Topics::UPDATE_SUCCESS
                | Topics::UPDATE_FAILED,
        ) && job_id.is_none()
        {
            return Err(());
        }

        // Check if more bits are set in `topic_mask` than
        // `subscribe_many` supports per invocation. If so, split into multiple
        // `subscribe_many` calls.

        let mut topics = heapless::Vec::new();

        if topic_mask.contains(Topics::NOTIFY) {
            if let Err(t) = topics.push(SubscribeTopic {
                topic_path: topic!("$aws/things/{}/jobs/notify", mqtt.client_id()),
                qos: QoS::AtLeastOnce,
            }) {
                mqtt.subscribe_many(topics).map_err(drop)?;
                topics = heapless::Vec::new();
                topics.push(t).map_err(drop)?;
            }
        }
        if topic_mask.contains(Topics::NOTIFY_NEXT) {
            if let Err(t) = topics.push(SubscribeTopic {
                topic_path: topic!("$aws/things/{}/jobs/notify-next", mqtt.client_id()),
                qos: QoS::AtLeastOnce,
            }) {
                mqtt.subscribe_many(topics).map_err(drop)?;
                topics = heapless::Vec::new();
                topics.push(t).map_err(drop)?;
            }
        }
        if topic_mask.contains(Topics::GET_ACCEPTED) {
            if let Err(t) = topics.push(SubscribeTopic {
                topic_path: topic!("$aws/things/{}/jobs/get/accepted", mqtt.client_id()),
                qos: QoS::AtLeastOnce,
            }) {
                mqtt.subscribe_many(topics).map_err(drop)?;
                topics = heapless::Vec::new();
                topics.push(t).map_err(drop)?;
            }
        }
        if topic_mask.contains(Topics::GET_REJECTED) {
            if let Err(t) = topics.push(SubscribeTopic {
                topic_path: topic!("$aws/things/{}/jobs/get/rejected", mqtt.client_id()),
                qos: QoS::AtLeastOnce,
            }) {
                mqtt.subscribe_many(topics).map_err(drop)?;
                topics = heapless::Vec::new();
                topics.push(t).map_err(drop)?;
            }
        }
        if topic_mask.contains(Topics::START_NEXT_ACCEPTED) {
            if let Err(t) = topics.push(SubscribeTopic {
                topic_path: topic!("$aws/things/{}/jobs/start-next/accepted", mqtt.client_id()),
                qos: QoS::AtLeastOnce,
            }) {
                mqtt.subscribe_many(topics).map_err(drop)?;
                topics = heapless::Vec::new();
                topics.push(t).map_err(drop)?;
            }
        }
        if topic_mask.contains(Topics::START_NEXT_REJECTED) {
            if let Err(t) = topics.push(SubscribeTopic {
                topic_path: topic!("$aws/things/{}/jobs/start-next/rejected", mqtt.client_id()),
                qos: QoS::AtLeastOnce,
            }) {
                mqtt.subscribe_many(topics).map_err(drop)?;
                topics = heapless::Vec::new();
                topics.push(t).map_err(drop)?;
            }
        }

        if let Some(job_id) = job_id {
            if topic_mask.contains(Topics::DESCRIBE_SUCCESS) {
                if let Err(t) = topics.push(SubscribeTopic {
                    topic_path: topic!(
                        "$aws/things/{}/jobs/{}/get/accepted",
                        mqtt.client_id(),
                        job_id
                    ),
                    qos: QoS::AtLeastOnce,
                }) {
                    mqtt.subscribe_many(topics).map_err(drop)?;
                    topics = heapless::Vec::new();
                    topics.push(t).map_err(drop)?;
                }
            }
            if topic_mask.contains(Topics::DESCRIBE_FAILED) {
                if let Err(t) = topics.push(SubscribeTopic {
                    topic_path: topic!(
                        "$aws/things/{}/jobs/{}/get/rejected",
                        mqtt.client_id(),
                        job_id
                    ),
                    qos: QoS::AtLeastOnce,
                }) {
                    mqtt.subscribe_many(topics).map_err(drop)?;
                    topics = heapless::Vec::new();
                    topics.push(t).map_err(drop)?;
                }
            }
            if topic_mask.contains(Topics::UPDATE_SUCCESS) {
                if let Err(t) = topics.push(SubscribeTopic {
                    topic_path: topic!(
                        "$aws/things/{}/jobs/{}/update/accepted",
                        mqtt.client_id(),
                        job_id
                    ),
                    qos: QoS::AtLeastOnce,
                }) {
                    mqtt.subscribe_many(topics).map_err(drop)?;
                    topics = heapless::Vec::new();
                    topics.push(t).map_err(drop)?;
                }
            }
            if topic_mask.contains(Topics::UPDATE_FAILED) {
                if let Err(t) = topics.push(SubscribeTopic {
                    topic_path: topic!(
                        "$aws/things/{}/jobs/{}/update/rejected",
                        mqtt.client_id(),
                        job_id
                    ),
                    qos: QoS::AtLeastOnce,
                }) {
                    mqtt.subscribe_many(topics).map_err(drop)?;
                    topics = heapless::Vec::new();
                    topics.push(t).map_err(drop)?;
                }
            }
        }

        mqtt.subscribe_many(topics).map_err(drop)?;

        Ok(())
    }

    pub fn unsubscribe<M: Mqtt>(
        mqtt: &M,
        topic_mask: Topics,
        job_id: Option<&str>,
    ) -> Result<(), ()> {
        if topic_mask.intersects(
            Topics::DESCRIBE_SUCCESS
                | Topics::DESCRIBE_FAILED
                | Topics::UPDATE_SUCCESS
                | Topics::UPDATE_FAILED,
        ) && job_id.is_none()
        {
            return Err(());
        }

        // Check if more bits are set in `topic_mask` than
        // `unsubscribe_many` supports per invocation. If so, split into multiple
        // `unsubscribe_many` calls.

        let mut topics = heapless::Vec::new();

        if topic_mask.contains(Topics::NOTIFY) {
            if let Err(t) = topics.push(topic!("$aws/things/{}/jobs/notify", mqtt.client_id())) {
                mqtt.unsubscribe_many(topics).map_err(drop)?;
                topics = heapless::Vec::new();
                topics.push(t).map_err(drop)?;
            }
        }
        if topic_mask.contains(Topics::NOTIFY_NEXT) {
            if let Err(t) = topics.push(topic!("$aws/things/{}/jobs/notify-next", mqtt.client_id()))
            {
                mqtt.unsubscribe_many(topics).map_err(drop)?;
                topics = heapless::Vec::new();
                topics.push(t).map_err(drop)?;
            }
        }
        if topic_mask.contains(Topics::GET_ACCEPTED) {
            if let Err(t) =
                topics.push(topic!("$aws/things/{}/jobs/get/accepted", mqtt.client_id()))
            {
                mqtt.unsubscribe_many(topics).map_err(drop)?;
                topics = heapless::Vec::new();
                topics.push(t).map_err(drop)?;
            }
        }
        if topic_mask.contains(Topics::GET_REJECTED) {
            if let Err(t) =
                topics.push(topic!("$aws/things/{}/jobs/get/rejected", mqtt.client_id()))
            {
                mqtt.unsubscribe_many(topics).map_err(drop)?;
                topics = heapless::Vec::new();
                topics.push(t).map_err(drop)?;
            }
        }
        if topic_mask.contains(Topics::START_NEXT_ACCEPTED) {
            if let Err(t) = topics.push(topic!(
                "$aws/things/{}/jobs/start-next/accepted",
                mqtt.client_id()
            )) {
                mqtt.unsubscribe_many(topics).map_err(drop)?;
                topics = heapless::Vec::new();
                topics.push(t).map_err(drop)?;
            }
        }
        if topic_mask.contains(Topics::START_NEXT_REJECTED) {
            if let Err(t) = topics.push(topic!(
                "$aws/things/{}/jobs/start-next/rejected",
                mqtt.client_id()
            )) {
                mqtt.unsubscribe_many(topics).map_err(drop)?;
                topics = heapless::Vec::new();
                topics.push(t).map_err(drop)?;
            }
        }

        if let Some(job_id) = job_id {
            if topic_mask.contains(Topics::DESCRIBE_SUCCESS) {
                if let Err(t) = topics.push(topic!(
                    "$aws/things/{}/jobs/{}/get/accepted",
                    mqtt.client_id(),
                    job_id
                )) {
                    mqtt.unsubscribe_many(topics).map_err(drop)?;
                    topics = heapless::Vec::new();
                    topics.push(t).map_err(drop)?;
                }
            }
            if topic_mask.contains(Topics::DESCRIBE_FAILED) {
                if let Err(t) = topics.push(topic!(
                    "$aws/things/{}/jobs/{}/get/rejected",
                    mqtt.client_id(),
                    job_id
                )) {
                    mqtt.unsubscribe_many(topics).map_err(drop)?;
                    topics = heapless::Vec::new();
                    topics.push(t).map_err(drop)?;
                }
            }
            if topic_mask.contains(Topics::UPDATE_SUCCESS) {
                if let Err(t) = topics.push(topic!(
                    "$aws/things/{}/jobs/{}/update/accepted",
                    mqtt.client_id(),
                    job_id
                )) {
                    mqtt.unsubscribe_many(topics).map_err(drop)?;
                    topics = heapless::Vec::new();
                    topics.push(t).map_err(drop)?;
                }
            }
            if topic_mask.contains(Topics::UPDATE_FAILED) {
                if let Err(t) = topics.push(topic!(
                    "$aws/things/{}/jobs/{}/update/rejected",
                    mqtt.client_id(),
                    job_id
                )) {
                    mqtt.unsubscribe_many(topics).map_err(drop)?;
                    topics = heapless::Vec::new();
                    topics.push(t).map_err(drop)?;
                }
            }
        }

        mqtt.unsubscribe_many(topics).map_err(drop)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use mqttrust::{SubscribeRequest, UnsubscribeRequest};

    use super::*;

    use crate::test::{MockMqtt, MqttRequest};

    #[test]
    fn splits_subscribe_all() {
        let mqtt = &MockMqtt::new();

        Jobs::subscribe(mqtt, Topics::all(), Some("test_job")).unwrap();

        assert_eq!(mqtt.tx.borrow_mut().len(), 2);
        assert_eq!(
            mqtt.tx.borrow_mut().pop_front(),
            Some(MqttRequest::Subscribe(SubscribeRequest {
                topics: heapless::Vec::from_slice(&[
                    SubscribeTopic {
                        topic_path: heapless::String::from("$aws/things/test_client/jobs/notify"),
                        qos: QoS::AtLeastOnce
                    },
                    SubscribeTopic {
                        topic_path: heapless::String::from(
                            "$aws/things/test_client/jobs/notify-next"
                        ),
                        qos: QoS::AtLeastOnce
                    },
                    SubscribeTopic {
                        topic_path: heapless::String::from(
                            "$aws/things/test_client/jobs/get/accepted"
                        ),
                        qos: QoS::AtLeastOnce
                    },
                    SubscribeTopic {
                        topic_path: heapless::String::from(
                            "$aws/things/test_client/jobs/get/rejected"
                        ),
                        qos: QoS::AtLeastOnce
                    },
                    SubscribeTopic {
                        topic_path: heapless::String::from(
                            "$aws/things/test_client/jobs/start-next/accepted"
                        ),
                        qos: QoS::AtLeastOnce
                    }
                ])
                .unwrap()
            }))
        );
        assert_eq!(
            mqtt.tx.borrow_mut().pop_front(),
            Some(MqttRequest::Subscribe(SubscribeRequest {
                topics: heapless::Vec::from_slice(&[
                    SubscribeTopic {
                        topic_path: heapless::String::from(
                            "$aws/things/test_client/jobs/start-next/rejected"
                        ),
                        qos: QoS::AtLeastOnce
                    },
                    SubscribeTopic {
                        topic_path: heapless::String::from(
                            "$aws/things/test_client/jobs/test_job/get/accepted"
                        ),
                        qos: QoS::AtLeastOnce
                    },
                    SubscribeTopic {
                        topic_path: heapless::String::from(
                            "$aws/things/test_client/jobs/test_job/get/rejected"
                        ),
                        qos: QoS::AtLeastOnce
                    },
                    SubscribeTopic {
                        topic_path: heapless::String::from(
                            "$aws/things/test_client/jobs/test_job/update/accepted"
                        ),
                        qos: QoS::AtLeastOnce
                    },
                    SubscribeTopic {
                        topic_path: heapless::String::from(
                            "$aws/things/test_client/jobs/test_job/update/rejected"
                        ),
                        qos: QoS::AtLeastOnce
                    }
                ])
                .unwrap()
            }))
        );
    }

    #[test]
    fn splits_unsubscribe_all() {
        let mqtt = &MockMqtt::new();

        Jobs::unsubscribe(mqtt, Topics::all(), Some("test_job")).unwrap();

        assert_eq!(mqtt.tx.borrow_mut().len(), 2);
        assert_eq!(
            mqtt.tx.borrow_mut().pop_front(),
            Some(MqttRequest::Unsubscribe(UnsubscribeRequest {
                topics: heapless::Vec::from_slice(&[
                    heapless::String::from("$aws/things/test_client/jobs/notify"),
                    heapless::String::from("$aws/things/test_client/jobs/notify-next"),
                    heapless::String::from("$aws/things/test_client/jobs/get/accepted"),
                    heapless::String::from("$aws/things/test_client/jobs/get/rejected"),
                    heapless::String::from("$aws/things/test_client/jobs/start-next/accepted"),
                ])
                .unwrap()
            }))
        );
        assert_eq!(
            mqtt.tx.borrow_mut().pop_front(),
            Some(MqttRequest::Unsubscribe(UnsubscribeRequest {
                topics: heapless::Vec::from_slice(&[
                    heapless::String::from("$aws/things/test_client/jobs/start-next/rejected"),
                    heapless::String::from("$aws/things/test_client/jobs/test_job/get/accepted"),
                    heapless::String::from("$aws/things/test_client/jobs/test_job/get/rejected"),
                    heapless::String::from("$aws/things/test_client/jobs/test_job/update/accepted"),
                    heapless::String::from("$aws/things/test_client/jobs/test_job/update/rejected")
                ])
                .unwrap()
            }))
        );
    }

    #[test]
    fn unnecessary_job_id() {
        let mqtt = &MockMqtt::new();
        Jobs::subscribe(mqtt, Topics::NOTIFY_NEXT, Some("test_job")).unwrap();
        assert_eq!(mqtt.tx.borrow_mut().len(), 1);

        Jobs::unsubscribe(mqtt, Topics::NOTIFY_NEXT, Some("test_job")).unwrap();
        assert_eq!(mqtt.tx.borrow_mut().len(), 2);
    }

    #[test]
    fn no_job_id() {
        let mqtt = &MockMqtt::new();
        Jobs::subscribe(mqtt, Topics::NOTIFY_NEXT, None).unwrap();
        assert_eq!(mqtt.tx.borrow_mut().len(), 1);

        Jobs::unsubscribe(mqtt, Topics::NOTIFY_NEXT, None).unwrap();
        assert_eq!(mqtt.tx.borrow_mut().len(), 2);
    }

    #[test]
    fn missing_job_id() {
        let mqtt = &MockMqtt::new();
        assert!(Jobs::subscribe(mqtt, Topics::DESCRIBE_SUCCESS, None).is_err());
        assert!(Jobs::unsubscribe(mqtt, Topics::DESCRIBE_SUCCESS, None).is_err());
    }
}
