#[allow(dead_code)]
#[allow(unused_variables)]
#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    struct User {
        id: u32,
        name: String,
    }

    #[test]
    fn test_1() -> Result<(), Box<dyn std::error::Error>> {
        let user = User {
            id: 100,
            name: "Rust".to_string(),
        };

        let json_user = serde_json::to_string(&user)?;
        println!("json_user: {}", json_user);

        Ok(())
    }
}
