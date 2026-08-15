use super::BadgeRule;

pub fn derive_badges(role_ids: &[String], rules: &[BadgeRule]) -> Vec<String> {
    let mut selected: Vec<&BadgeRule> = rules
        .iter()
        .filter(|rule| role_ids.iter().any(|role_id| role_id == &rule.role_id))
        .collect();

    selected.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.badge_id.cmp(&right.badge_id))
    });

    selected
        .into_iter()
        .map(|rule| rule.badge_id.clone())
        .fold(Vec::new(), |mut badges, badge| {
            if !badges.contains(&badge) {
                badges.push(badge);
            }
            badges
        })
}
