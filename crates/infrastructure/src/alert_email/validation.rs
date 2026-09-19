use application::alerts::AlertEmailMessage;

pub(super) fn valid(message: &AlertEmailMessage) -> bool {
    let key = &message.idempotency_key;
    !key.is_empty()
        && key.len() <= 256
        && key.bytes().all(|byte| byte.is_ascii_graphic())
        && address(&message.from_email)
        && address(&message.recipient_email)
        && login_url(&message.login_url)
}

pub(super) fn address(value: &str) -> bool {
    if value.len() > 320 || value.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return false;
    }
    value.split_once('@').is_some_and(|(local, domain)| {
        !local.is_empty() && !domain.is_empty() && !domain.contains('@')
    })
}

pub(super) fn login_url(value: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(value) else {
        return false;
    };
    let loopback = url.host_str().is_some_and(|host| {
        host == "localhost"
            || host
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    });
    (url.scheme() == "https" || url.scheme() == "http" && loopback)
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && matches!(url.path(), "/" | "/login")
}
