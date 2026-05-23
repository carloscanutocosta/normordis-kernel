use support_ids::{new_id, new_id_string};

#[test]
fn generated_ids_are_not_empty() {
    assert!(!new_id().as_str().is_empty());
    assert!(!new_id_string().is_empty());
}
