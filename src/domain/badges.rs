use super::{BadgeGroup, BadgeRule};

pub fn derive_badges(role_ids: &[String], rules: &[BadgeRule]) -> Vec<String> {
    [BadgeGroup::Career, BadgeGroup::Team]
        .into_iter()
        .filter_map(|group| best_rule(role_ids, rules, group))
        .map(|rule| rule.badge_id.clone())
        .collect()
}

fn best_rule<'a>(
    role_ids: &[String],
    rules: &'a [BadgeRule],
    group: BadgeGroup,
) -> Option<&'a BadgeRule> {
    rules
        .iter()
        .filter(|rule| {
            rule.group == group && role_ids.iter().any(|role_id| role_id == &rule.role_id)
        })
        .min_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| left.badge_id.cmp(&right.badge_id))
        })
}
