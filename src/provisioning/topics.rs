use core::fmt::Display;
use core::fmt::Write;
use core::str::FromStr;

use embedded_mqtt::QoS;
use heapless::String;

use super::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Direction {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PayloadFormat {
    #[cfg(feature = "provision_cbor")]
    Cbor,
    Json,
}

impl Display for PayloadFormat {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            #[cfg(feature = "provision_cbor")]
            Self::Cbor => write!(f, "cbor"),
            Self::Json => write!(f, "json"),
        }
    }
}

impl FromStr for PayloadFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            #[cfg(feature = "provision_cbor")]
            "cbor" => Ok(Self::Cbor),
            "json" => Ok(Self::Json),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Topic<'a> {
    // ---- Outgoing Topics
    /// `$aws/provisioning-templates/<templateName>/provision/<payloadFormat>`
    RegisterThing(&'a str, PayloadFormat),

    /// $aws/certificates/create/<payloadFormat>
    CreateKeysAndCertificate(PayloadFormat),

    /// $aws/certificates/create-from-csr/<payloadFormat>
    CreateCertificateFromCsr(PayloadFormat),

    // ---- Incoming Topics
    /// `$aws/provisioning-templates/<templateName>/provision/<payloadFormat>/accepted`
    RegisterThingAccepted(&'a str, PayloadFormat),

    /// `$aws/provisioning-templates/<templateName>/provision/<payloadFormat>/rejected`
    RegisterThingRejected(&'a str, PayloadFormat),

    /// `$aws/certificates/create/<payloadFormat>/accepted`
    CreateKeysAndCertificateAccepted(PayloadFormat),

    /// `$aws/certificates/create/<payloadFormat>/rejected`
    CreateKeysAndCertificateRejected(PayloadFormat),

    /// `$aws/certificates/create-from-csr/<payloadFormat>/accepted`
    CreateCertificateFromCsrAccepted(PayloadFormat),

    /// `$aws/certificates/create-from-csr/<payloadFormat>/rejected`
    CreateCertificateFromCsrRejected(PayloadFormat),
}

impl<'a> Topic<'a> {
    const CERT_PREFIX: &'static str = "$aws/certificates";
    const PROVISIONING_PREFIX: &'static str = "$aws/provisioning-templates";

    pub fn check(s: &'a str) -> bool {
        s.starts_with(Self::CERT_PREFIX) || s.starts_with(Self::PROVISIONING_PREFIX)
    }

    pub fn from_str(s: &'a str) -> Option<Self> {
        let tt = s.splitn(6, '/').collect::<heapless::Vec<&str, 6>>();
        match (tt.get(0), tt.get(1)) {
            (Some(&"$aws"), Some(&"provisioning-templates")) => {
                // This is a register thing topic, now figure out which one.

                match (tt.get(2), tt.get(3), tt.get(4), tt.get(5)) {
                    (
                        Some(template_name),
                        Some(&"provision"),
                        Some(payload_format),
                        Some(&"accepted"),
                    ) => Some(Topic::RegisterThingAccepted(
                        *template_name,
                        PayloadFormat::from_str(payload_format).ok()?,
                    )),
                    (
                        Some(template_name),
                        Some(&"provision"),
                        Some(payload_format),
                        Some(&"rejected"),
                    ) => Some(Topic::RegisterThingRejected(
                        *template_name,
                        PayloadFormat::from_str(payload_format).ok()?,
                    )),
                    _ => None,
                }
            }
            (Some(&"$aws"), Some(&"certificates")) => {
                // This is a register thing topic, now figure out which one.

                match (tt.get(2), tt.get(3), tt.get(4)) {
                    (Some(&"create"), Some(payload_format), Some(&"accepted")) => {
                        Some(Topic::CreateKeysAndCertificateAccepted(
                            PayloadFormat::from_str(payload_format).ok()?,
                        ))
                    }
                    (Some(&"create"), Some(payload_format), Some(&"rejected")) => {
                        Some(Topic::CreateKeysAndCertificateRejected(
                            PayloadFormat::from_str(payload_format).ok()?,
                        ))
                    }
                    (Some(&"create-from-csr"), Some(payload_format), Some(&"accepted")) => {
                        Some(Topic::CreateCertificateFromCsrAccepted(
                            PayloadFormat::from_str(payload_format).ok()?,
                        ))
                    }
                    (Some(&"create-from-csr"), Some(payload_format), Some(&"rejected")) => {
                        Some(Topic::CreateCertificateFromCsrRejected(
                            PayloadFormat::from_str(payload_format).ok()?,
                        ))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub fn direction(&self) -> Direction {
        if matches!(
            self,
            Topic::RegisterThing(_, _)
                | Topic::CreateKeysAndCertificate(_)
                | Topic::CreateCertificateFromCsr(_)
        ) {
            Direction::Outgoing
        } else {
            Direction::Incoming
        }
    }

    pub fn format<const L: usize>(&self) -> Result<String<L>, Error> {
        let mut topic_path = String::new();
        match self {
            Self::RegisterThing(template_name, payload_format) => {
                topic_path.write_fmt(format_args!(
                    "{}/{}/provision/{}",
                    Self::PROVISIONING_PREFIX,
                    template_name,
                    payload_format,
                ))
            }
            Topic::RegisterThingAccepted(template_name, payload_format) => {
                topic_path.write_fmt(format_args!(
                    "{}/{}/provision/{}/accepted",
                    Self::PROVISIONING_PREFIX,
                    template_name,
                    payload_format,
                ))
            }
            Topic::RegisterThingRejected(template_name, payload_format) => {
                topic_path.write_fmt(format_args!(
                    "{}/{}/provision/{}/rejected",
                    Self::PROVISIONING_PREFIX,
                    template_name,
                    payload_format,
                ))
            }

            Topic::CreateKeysAndCertificate(payload_format) => topic_path.write_fmt(format_args!(
                "{}/create/{}",
                Self::CERT_PREFIX,
                payload_format,
            )),

            Topic::CreateKeysAndCertificateAccepted(payload_format) => topic_path.write_fmt(
                format_args!("{}/create/{}/accepted", Self::CERT_PREFIX, payload_format),
            ),
            Topic::CreateKeysAndCertificateRejected(payload_format) => topic_path.write_fmt(
                format_args!("{}/create/{}/rejected", Self::CERT_PREFIX, payload_format),
            ),

            Topic::CreateCertificateFromCsr(payload_format) => topic_path.write_fmt(format_args!(
                "{}/create-from-csr/{}",
                Self::CERT_PREFIX,
                payload_format,
            )),
            Topic::CreateCertificateFromCsrAccepted(payload_format) => topic_path.write_fmt(
                format_args!("{}/create-from-csr/{}", Self::CERT_PREFIX, payload_format),
            ),
            Topic::CreateCertificateFromCsrRejected(payload_format) => topic_path.write_fmt(
                format_args!("{}/create-from-csr/{}", Self::CERT_PREFIX, payload_format),
            ),
        }
        .map_err(|_| Error::Overflow)?;

        Ok(topic_path)
    }
}

#[derive(Default)]
pub struct Subscribe<'a, const N: usize> {
    topics: heapless::Vec<(Topic<'a>, QoS), N>,
}

impl<'a, const N: usize> Subscribe<'a, N> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn topic(self, topic: Topic<'a>, qos: QoS) -> Self {
        // Ignore attempts to subscribe to outgoing topics
        if topic.direction() != Direction::Incoming {
            return self;
        }

        if self.topics.iter().any(|(t, _)| t == &topic) {
            return self;
        }

        let mut topics = self.topics;
        topics.push((topic, qos)).ok();

        Self { topics }
    }

    pub fn topics<const MAX_LEN: usize>(
        self,
    ) -> Result<heapless::Vec<(heapless::String<MAX_LEN>, QoS), N>, Error> {
        self.iter()
            .map(|(topic, qos)| Ok((topic.format()?, *qos)))
            .collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = &(Topic<'a>, QoS)> {
        self.topics.iter()
    }
}

// #[derive(Default)]
// pub struct Unsubscribe<'a, const N: usize> {
//     topics: heapless::Vec<Topic<'a>, N>,
// }

// impl<'a, const N: usize> Unsubscribe<'a, N> {
//     pub fn new() -> Self {
//         Self::default()
//     }

//     pub fn topic(self, topic: Topic<'a>) -> Self {
//         // Ignore attempts to subscribe to outgoing topics
//         if topic.direction() != Direction::Incoming {
//             return self;
//         }

//         if self.topics.iter().any(|t| t == &topic) {
//             return self;
//         }

//         let mut topics = self.topics;
//         topics.push(topic).ok();
//         Self { topics }
//     }

//     pub fn topics(self) -> Result<heapless::Vec<heapless::String<256>, N>, Error> {
//         self.topics
//             .iter()
//             .map(|topic| topic.clone().format())
//             .collect()
//     }

//     // pub fn send<M: Mqtt>(self, mqtt: &M) -> Result<(), Error> {
//     //     if self.topics.is_empty() {
//     //         return Ok(());
//     //     }

//     //     let topic_paths = self.topics()?;
//     //     let topics: heapless::Vec<_, N> = topic_paths.iter().map(|s| s.as_str()).collect();

//     //     for t in topics.chunks(5) {
//     //         // mqtt.unsubscribe(t)?;
//     //     }

//     //     Ok(())
//     // }
// }
