pub(super) fn complete_standard_endpoint(
    endpoint: &str,
    standard_path: &str,
    standard_prefixes: &[&str],
) -> String {
    let trimmed = endpoint.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let (without_query, query) = split_query(trimmed);
    let without_query = without_query.trim_end_matches('/');
    if without_query.is_empty() {
        return String::new();
    }

    let Some(path_start) = path_start(without_query) else {
        return format!("{}{}{}", without_query, standard_path, query);
    };

    let path = &without_query[path_start..];
    if path.is_empty() || path == "/" {
        return format!("{}{}{}", &without_query[..path_start], standard_path, query);
    }

    if let Some(prefix) = find_standard_endpoint_suffix(path, standard_prefixes) {
        let base_end = without_query.len() - prefix.len();
        return format!("{}{}{}", &without_query[..base_end], standard_path, query);
    }

    format!("{}{}", without_query, query)
}

fn find_standard_endpoint_suffix<'a>(path: &'a str, standard_prefixes: &[&str]) -> Option<&'a str> {
    path.match_indices('/')
        .map(|(index, _)| &path[index..])
        .filter(|suffix| {
            standard_prefixes
                .iter()
                .any(|prefix| prefix.starts_with(*suffix))
        })
        .max_by_key(|suffix| suffix.len())
}

fn split_query(endpoint: &str) -> (&str, &str) {
    if let Some(index) = endpoint.find('?') {
        (&endpoint[..index], &endpoint[index..])
    } else {
        (endpoint, "")
    }
}

fn path_start(endpoint: &str) -> Option<usize> {
    if let Some(scheme_index) = endpoint.find("://") {
        let authority_start = scheme_index + 3;
        return endpoint[authority_start..]
            .find('/')
            .map(|relative_index| authority_start + relative_index);
    }

    endpoint.find('/')
}
