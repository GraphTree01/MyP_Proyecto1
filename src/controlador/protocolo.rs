use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum Message {
    #[serde(rename = "IDENTIFY")]
    Identify {
        username: String,
    },

    Response {
        operation: Operation,
        result: Result,
        extra: String,
    },
    
    New_User {
        username: String,
    },
}

#[derive(Serialize, Deserialize)]
enum Operation {
    #[serde(rename = "IDENTIFY")]
    Identify,
}

#[derive(Serialize, Deserialize)]
enum Result {
    SUCCESS,
    USER_ALREADY_EXIST,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prueba_identify() {
        let mensaje = Message::Identify {
            username: String::from("Kimberly"),
        };

        let json = serde_json::to_string(&mensaje).unwrap();

        assert_eq!(
            json,
            r#"{"type":"IDENTIFY","username":"Kimberly"}"#
            );
    }
}
