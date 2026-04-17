use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tower_cookies::{Cookie, Cookies};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FlashData {
    pub kind: String,
    pub message: String,
}

#[derive(Deserialize)]
struct ValuedMessage<T> {
    #[serde(rename = "_")]
    value: T,
}

#[derive(Serialize)]
struct ValuedMessageRef<'a, T> {
    #[serde(rename = "_")]
    value: &'a T,
}

const FLASH_COOKIE_NAME: &str = "_flash";

pub fn get_flash_cookie<T>(cookies: &mut Cookies) -> Option<T>
where
    T: DeserializeOwned,
{
    let cookie = cookies.get(FLASH_COOKIE_NAME)?;
    let value = match serde_json::from_str::<ValuedMessage<T>>(cookie.value()) {
        Ok(msg) => Some(msg.value),
        Err(_) => None,
    };

    // Remove the cookie so it only shows once
    let mut removal_cookie = Cookie::new(FLASH_COOKIE_NAME, "");
    removal_cookie.set_path("/");
    removal_cookie.make_removal();
    cookies.add(removal_cookie);

    value
}

pub type PostResponse = (StatusCode, HeaderMap);

pub fn post_response<T>(cookies: &mut Cookies, data: T) -> PostResponse
where
    T: Serialize,
{
    let valued_message_ref = ValuedMessageRef { value: &data };

    let mut cookie = Cookie::new(
        FLASH_COOKIE_NAME,
        serde_json::to_string(&valued_message_ref).unwrap(),
    );
    cookie.set_path("/");
    cookies.add(cookie);

    let mut header = HeaderMap::new();
    header.insert(header::LOCATION, HeaderValue::from_static("/post"));

    (StatusCode::SEE_OTHER, header)
}

pub fn admin_redirect_response<T>(cookies: &mut Cookies, data: T, path: &str) -> PostResponse
where
    T: Serialize,
{
    let valued_message_ref = ValuedMessageRef { value: &data };

    let mut cookie = Cookie::new(
        FLASH_COOKIE_NAME,
        serde_json::to_string(&valued_message_ref).unwrap(),
    );
    cookie.set_path("/");
    cookies.add(cookie);

    let mut header = HeaderMap::new();
    header.insert(header::LOCATION, HeaderValue::from_str(path).unwrap());

    (StatusCode::SEE_OTHER, header)
}
