use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

// use crate::director_executor::DirectorAction;

/// The `config::UpdateObject` message is used to create new object in the master. It contains at
/// least the following values:
///
///     * The name of the object is composed like <hostname>[!<service_name>] for checkable objects.
///     * The type of the object is either "Host" or "Service" for the two checkable Objects.
///     * The version is a number and should always be 0.0 to show that is is the first verison.
///     * The zone
///
/// Further more has the object a packed trait which should always be 'director' if the master icinga
/// director is used to avoid conflicts on the next deploy. However if that is the case, the objects
/// also needs to be persisted in the director, so it doesn't get over written by the next deploy.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct UpdateObjectParams {
    pub name: String,
    #[serde(rename = "type")]
    pub object_type: String,
    version: f64,

    #[serde(skip_serializing_if = "Option::is_none")]
    config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modified_attributes: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    original_attributes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    package: Option<String>,
}

pub struct UpdateObjectParamsBuilder {
    host: String,
    service: Option<String>,
    pub zone: Option<String>,
    pub modified_attributes: Option<Map<String, Value>>,
    pub original_attributes: Option<Vec<String>>,
}

impl UpdateObjectParams {
    pub fn create_host(host: String) -> UpdateObjectParamsBuilder {
        UpdateObjectParamsBuilder {
            host,
            service: None,
            zone: None,
            modified_attributes: None,
            original_attributes: None,
        }
    }

    pub fn create_service(host: String, service: String) -> UpdateObjectParamsBuilder {
        UpdateObjectParamsBuilder {
            host,
            service: Some(service),
            zone: None,
            modified_attributes: None,
            original_attributes: None,
        }
    }
}

impl UpdateObjectParamsBuilder {
    pub fn build(&self) -> UpdateObjectParams {
        match self.service.as_deref() {
            None => self.build_host(),
            Some(service) => self.build_service(service),
        }
    }

    fn build_host(&self) -> UpdateObjectParams {
        let zone = match self.zone.as_deref() {
            None => "".to_owned(),
            Some(zone) => format!("\tzone={}\n", zone)
        };
        let config = format!(
            "object Host \"{}\" {{\n\tcheck_command = \"dummy_alive\"\n{}}}",
            self.host,
            zone
        );

        UpdateObjectParams {
            name: self.host.to_string(),
            object_type: "Host".to_string(),
            version: 0.0,
            config: Some(config),
            modified_attributes: self.modified_attributes.clone(),
            original_attributes: self.original_attributes.clone(),
            package: Some("director".to_string()),
        }
    }

    fn build_service(&self, service: &str) -> UpdateObjectParams {
        let config = format!(
            "object Service \"{}\" {{\n\thost_name = \"{}\"\n\tcheck_command = \"dummy_alive\"\n}}",
            service, self.host
        );

        UpdateObjectParams {
            name: format!("{}!{}", self.host, service),
            object_type: "Service".to_string(),
            version: 0.0,
            config: Some(config),
            modified_attributes: self.modified_attributes.clone(),
            original_attributes: self.original_attributes.clone(),
            package: Some("director".to_string()),
        }
    }

    // pub fn build_director_action(&self) -> DirectorAction {
    //     match self.service.as_deref() {
    //         None => self.build_director_host_action(),
    //         Some(service) => self.build_director_satellite_action(service),
    //     }
    // }
    //
    // pub fn build_director_host_action(&self) -> DirectorAction {
    //     DirectorAction {
    //         name: "host".to_string(),
    //         payload: hashmap!(
    //             "object_name".to_string()   => json!(self.host),
    //             "object_type".to_string()   => json!("Object"),
    //             "address".to_string()       => json!("127.0.0.1"),
    //             "check_command".to_string() => json!("dummy_alive"),
    //             "zone".to_string()          => json!(self.zone.to_owned()),
    //         ),
    //     }
    // }
    //
    // pub fn build_director_satellite_action(&self, service: &str) -> DirectorAction {
    //     DirectorAction {
    //         name: "service".to_string(),
    //         payload: hashmap!(
    //             "object_name".to_string() => json!(service),
    //             "object_type".to_string() => json!("Object"),
    //             "host".to_string() => json!(self.host),
    //             "check_command".to_string() => json!("dummy_alive"),
    //         ),
    //     }
    // }
}
