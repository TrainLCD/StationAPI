pub fn delete_option_from_string_vec(vec: Vec<Option<String>>) -> Vec<String> {
    vec.into_iter()
        .filter_map(|opt| {
            if opt.is_some() {
                return opt;
            }
            None
        })
        .collect()
}
