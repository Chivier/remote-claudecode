use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub user_id: i64,
    pub username: String,
    pub exp: usize,
    pub iat: usize,
}

pub fn generate_token(user_id: i64, username: &str, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let exp = now + Duration::days(7);

    let claims = Claims {
        user_id,
        username: username.to_string(),
        exp: exp.timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

/// Check if a token is past its half-life and should be refreshed
pub fn should_refresh(claims: &Claims) -> bool {
    let now = Utc::now().timestamp() as usize;
    let half_life = (claims.exp - claims.iat) / 2;
    now > claims.iat + half_life
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-key-for-jwt";

    #[test]
    fn test_generate_and_verify_token() {
        let token = generate_token(1, "alice", TEST_SECRET).unwrap();
        assert!(!token.is_empty());

        let claims = verify_token(&token, TEST_SECRET).unwrap();
        assert_eq!(claims.user_id, 1);
        assert_eq!(claims.username, "alice");
    }

    #[test]
    fn test_verify_with_wrong_secret() {
        let token = generate_token(1, "alice", TEST_SECRET).unwrap();
        let result = verify_token(&token, "wrong-secret");
        assert!(result.is_err());
    }

    #[test]
    fn test_token_expiration() {
        let token = generate_token(1, "alice", TEST_SECRET).unwrap();
        let claims = verify_token(&token, TEST_SECRET).unwrap();
        // Token should expire 7 days from now
        let now = Utc::now().timestamp() as usize;
        let seven_days = 7 * 24 * 60 * 60;
        assert!(claims.exp > now);
        assert!(claims.exp <= now + seven_days + 1);
    }

    #[test]
    fn test_should_refresh_fresh_token() {
        let claims = Claims {
            user_id: 1,
            username: "alice".to_string(),
            iat: Utc::now().timestamp() as usize,
            exp: (Utc::now() + Duration::days(7)).timestamp() as usize,
        };
        assert!(!should_refresh(&claims));
    }

    #[test]
    fn test_should_refresh_old_token() {
        let claims = Claims {
            user_id: 1,
            username: "alice".to_string(),
            iat: (Utc::now() - Duration::days(5)).timestamp() as usize,
            exp: (Utc::now() + Duration::days(2)).timestamp() as usize,
        };
        assert!(should_refresh(&claims));
    }

    #[test]
    fn test_verify_invalid_token() {
        let result = verify_token("not.a.valid.token", TEST_SECRET);
        assert!(result.is_err());
    }
}
