use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Mensaje {
    #[serde(rename = "IDENTIFY")]
    Identify {
        username: String,
    },


    #[serde(rename = "RESPONSE")]
    Response {
        operation: Operation,
        result: Resultado,
        #[serde(skip_serializing_if = "Option::is_none")]
        extra: Option<String>,
    },
    
    #[serde(rename = "NEW_USER")]
    NewUser {
        username: String,
    },
}

#[derive(Serialize, Deserialize)]
pub enum Operation {
    #[serde(rename = "IDENTIFY")]
    Identify,
    #[serde(rename ="INVALID")]
    Invalid,
}

#[derive(Serialize, Deserialize)]
pub enum Resultado {
    #[serde(rename = "SUCCESS")]
    Success,
    #[serde(rename = "USER_ALREADY_EXIST")]
    UserAlreadyExist,
    #[serde(rename ="NOT_IDENTIFIED")]
    NotIdentified,
    #[serde(rename ="INVALID")]
    Invalid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prueba_identify() {
        let mensaje = Mensaje::Identify {
            username: String::from("Kimberly"),
        };

        let json = serde_json::to_string(&mensaje).unwrap();

        assert_eq!(
            json,
            r#"{"type":"IDENTIFY","username":"Kimberly"}"#
            );
    }

    #[test]
    fn prueba_response_success() {
        let mensaje = Mensaje::Response {
            operation: Operation::Identify,
            result: Resultado::Success,
            extra: Some(String::from("Kimberly")),
        };

        let json = serde_json::to_string(&mensaje).unwrap();

        assert_eq!(
            json,
            r#"{"type":"RESPONSE","operation":"IDENTIFY","result":"SUCCESS","extra":"Kimberly"}"#
            );
    }

    #[test]
    fn prueba_response_user_already_exist() {
        let mensaje = Mensaje::Response {
            operation: Operation::Identify,
            result: Resultado::UserAlreadyExist,
            extra: Some(String::from("Kimberly")),
        };

        let json = serde_json::to_string(&mensaje).unwrap();

        assert_eq!(
            json,
            r#"{"type":"RESPONSE","operation":"IDENTIFY","result":"USER_ALREADY_EXIST","extra":"Kimberly"}"#
            );
    }

}
