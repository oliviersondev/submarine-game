use shared::CrewRole;

pub fn parse_role(value: &str) -> Option<CrewRole> {
    match value.to_ascii_lowercase().as_str() {
        "captain" => Some(CrewRole::Captain),
        "pilot" => Some(CrewRole::Pilot),
        "sonar" => Some(CrewRole::Sonar),
        "engineer" => Some(CrewRole::Engineer),
        "weapons" => Some(CrewRole::Weapons),
        _ => None,
    }
}

fn role_from_query(query: &str) -> Option<CrewRole> {
    query
        .trim_start_matches('?')
        .split('&')
        .filter_map(|parameter| parameter.split_once('='))
        .find(|(key, _)| key.eq_ignore_ascii_case("role"))
        .and_then(|(_, value)| parse_role(value))
}

#[cfg(target_arch = "wasm32")]
pub fn current_role() -> CrewRole {
    web_sys::window()
        .and_then(|window| window.location().search().ok())
        .as_deref()
        .and_then(role_from_query)
        .unwrap_or(CrewRole::Captain)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn current_role() -> CrewRole {
    std::env::var("ROLE")
        .ok()
        .as_deref()
        .and_then(parse_role)
        .unwrap_or(CrewRole::Captain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_roles_case_insensitively() {
        assert_eq!(parse_role("captain"), Some(CrewRole::Captain));
        assert_eq!(parse_role("PILOT"), Some(CrewRole::Pilot));
        assert_eq!(parse_role("Sonar"), Some(CrewRole::Sonar));
        assert_eq!(parse_role("engineer"), Some(CrewRole::Engineer));
        assert_eq!(parse_role("weapons"), Some(CrewRole::Weapons));
    }

    #[test]
    fn rejects_unknown_roles() {
        assert_eq!(parse_role("navigator"), None);
        assert_eq!(parse_role(""), None);
    }

    #[test]
    fn extracts_role_from_query_parameters() {
        assert_eq!(role_from_query("?role=pilot"), Some(CrewRole::Pilot));
        assert_eq!(
            role_from_query("?debug=true&ROLE=weapons"),
            Some(CrewRole::Weapons)
        );
        assert_eq!(role_from_query("?debug=true"), None);
    }
}
