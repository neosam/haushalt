//! Signal-free presentation logic for the task card.
//!
//! Companion to `task_form_model.rs`: the form translates "what am I creating?" into flags, this
//! module translates the resulting flags back into "what does this card offer?". Plain data and
//! pure functions — no Leptos, no signals, no DOM — so it runs under `#[test]` on the host.
//!
//! The archetype decides *wording and colour*, never the mechanics. What somebody may actually do
//! still comes from `TaskWithStatus::can_complete` / `can_uncomplete`.

use shared::Archetype;

/// Visual weight of the primary action button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionStyle {
    /// Something worth doing — the normal case.
    Primary,
    /// Logging a lapse of one's own bad habit.
    Warn,
    /// Reporting somebody else's lapse.
    Danger,
}

impl ActionStyle {
    /// `task-action-btn` carries the 44 px touch target the design asks for. It sits only on the
    /// card's primary action rather than on `.btn` itself, so the rest of the app keeps its
    /// current button metrics.
    pub fn css_class(&self) -> &'static str {
        match self {
            ActionStyle::Primary => "btn task-action-btn btn-primary",
            ActionStyle::Warn => "btn task-action-btn btn-warn",
            ActionStyle::Danger => "btn task-action-btn btn-danger",
        }
    }
}

/// What the action area of a card offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardAction {
    /// `−` / count / `+` — needed whenever a target has to be worked towards step by step.
    Counter,
    /// One button that says what it does. A separate undo appears only when there is something
    /// to undo and the user is allowed to.
    Single {
        label_key: &'static str,
        style: ActionStyle,
    },
    /// Somebody else logged something the user is not allowed to clear. This is the case the
    /// mockup singles out: today it renders as a permanently disabled `−` whose explanation
    /// hides in a `title` attribute — invisible on touch devices. The card states the reason
    /// instead of offering a dead button.
    Locked,
    /// The user may not check this task off at all — the count is shown, nothing else.
    ReadOnly,
}

/// What the card's action area offers, given the task's flags and the user's permissions.
///
/// The order of the checks matters: permission questions come before presentation ones, because
/// a button the user cannot use must never be rendered just because the type would suggest it.
pub fn card_action(
    archetype: Archetype,
    target_count: i32,
    completions: i32,
    can_uncomplete: bool,
    is_completable_by_user: bool,
) -> CardAction {
    if !is_completable_by_user {
        return CardAction::ReadOnly;
    }

    // Something is logged that this user may not take back. Saying so beats a dead button.
    if completions > 0 && !can_uncomplete {
        return CardAction::Locked;
    }

    // The bauform follows the target, not the type: a shared task with a target of 3 still needs
    // the step-by-step counter, and a routine with a target of 1 does not.
    if target_count > 1 {
        return CardAction::Counter;
    }

    let (label_key, style) = match archetype {
        Archetype::OneOff => ("task_card.action.oneoff", ActionStyle::Primary),
        Archetype::Routine => ("task_card.action.routine", ActionStyle::Primary),
        Archetype::Shared => ("task_card.action.shared", ActionStyle::Primary),
        Archetype::Bonus => ("task_card.action.bonus", ActionStyle::Primary),
        Archetype::BadHabit => ("task_card.action.bad_habit", ActionStyle::Warn),
        Archetype::Maintenance => ("task_card.action.maintenance", ActionStyle::Danger),
    };

    CardAction::Single { label_key, style }
}

/// CSS modifier carrying the archetype's colour accent on the left edge of the card.
pub fn accent_class(archetype: Archetype) -> &'static str {
    match archetype {
        Archetype::OneOff => "task-item--oneoff",
        Archetype::Routine => "task-item--routine",
        Archetype::Shared => "task-item--shared",
        Archetype::BadHabit => "task-item--bad-habit",
        Archetype::Maintenance => "task-item--maintenance",
        Archetype::Bonus => "task-item--bonus",
    }
}

/// Icon and i18n key for the type badge in the meta line.
///
/// `None` for `OneOff` and `Routine`: they are what a task looks like by default, so labelling
/// them adds noise instead of information. The other four each carry a rule worth naming.
pub fn type_badge(archetype: Archetype) -> Option<(&'static str, &'static str)> {
    match archetype {
        Archetype::OneOff | Archetype::Routine => None,
        Archetype::Shared => Some(("👥", "task_card.badge.shared")),
        Archetype::BadHabit => Some(("⚠️", "task_card.badge.bad_habit")),
        Archetype::Maintenance => Some(("🛡️", "task_card.badge.maintenance")),
        Archetype::Bonus => Some(("🎁", "task_card.badge.bonus")),
    }
}

/// How long the card waits for further taps before it talks to the server.
///
/// Long enough that a burst of taps collapses into one round trip, short enough that a single
/// tap still feels immediate — the count itself updates without waiting for any of this.
pub const COUNTER_FLUSH_MS: u64 = 500;

/// The count to display while taps are still pending.
///
/// Clamped at zero: the pending delta may briefly undershoot when somebody taps `−` more often
/// than there are completions, and a card must never show "-1 ×".
pub fn effective_completions(server_completions: i32, pending_delta: i32) -> i32 {
    (server_completions + pending_delta).max(0)
}

/// Whether `+` is still available, judged on the optimistic count rather than the server's.
///
/// Mirrors `TaskWithStatus::can_complete`, which computes the same rule from `completions_today`.
/// Without this the button would stay enabled through a whole burst of taps and let the user
/// overshoot a target the server then rejects.
pub fn can_complete_pending(
    is_completable_by_user: bool,
    allow_exceed_target: bool,
    target_count: i32,
    effective_completions: i32,
) -> bool {
    let target_met = target_count > 0 && effective_completions >= target_count;
    is_completable_by_user && (allow_exceed_target || !target_met)
}

/// Whether `−` is still available. There has to be something left to take back.
pub fn can_undo_pending(can_uncomplete: bool, effective_completions: i32) -> bool {
    can_uncomplete && effective_completions > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Archetype; 6] = [
        Archetype::OneOff,
        Archetype::Routine,
        Archetype::Shared,
        Archetype::BadHabit,
        Archetype::Maintenance,
        Archetype::Bonus,
    ];

    /// The case the mockup is about: the assignee of a maintenance task may not clear what
    /// somebody else logged. Today that renders as a disabled button.
    #[test]
    fn locked_when_something_is_logged_that_cannot_be_undone() {
        let action = card_action(Archetype::Maintenance, 1, 2, false, true);
        assert_eq!(action, CardAction::Locked);
    }

    /// Without anything logged there is nothing to lock — the button must still be offered.
    #[test]
    fn not_locked_while_nothing_is_logged() {
        let action = card_action(Archetype::Maintenance, 1, 0, false, true);
        assert!(matches!(action, CardAction::Single { .. }));
    }

    #[test]
    fn read_only_wins_over_everything() {
        for archetype in ALL {
            assert_eq!(
                card_action(archetype, 3, 2, true, false),
                CardAction::ReadOnly,
                "{archetype:?} offered an action to a user who may not complete it"
            );
        }
    }

    /// The bauform follows the target, not the type.
    #[test]
    fn a_target_above_one_keeps_the_counter_for_every_type() {
        for archetype in ALL {
            assert_eq!(
                card_action(archetype, 3, 0, true, true),
                CardAction::Counter,
                "{archetype:?} lost its counter"
            );
        }
    }

    #[test]
    fn a_target_of_one_yields_a_single_button() {
        assert!(matches!(
            card_action(Archetype::Routine, 1, 0, true, true),
            CardAction::Single { .. }
        ));
    }

    /// A bonus task has a target of 0 — it must not fall into the counter branch.
    #[test]
    fn bonus_yields_a_single_button() {
        assert_eq!(
            card_action(Archetype::Bonus, 0, 5, true, true),
            CardAction::Single {
                label_key: "task_card.action.bonus",
                style: ActionStyle::Primary
            }
        );
    }

    #[test]
    fn logging_a_lapse_is_styled_as_a_warning_not_a_success() {
        let action = card_action(Archetype::BadHabit, 1, 0, true, true);
        assert_eq!(
            action,
            CardAction::Single {
                label_key: "task_card.action.bad_habit",
                style: ActionStyle::Warn
            }
        );
    }

    #[test]
    fn reporting_someone_elses_lapse_is_styled_as_danger() {
        let action = card_action(Archetype::Maintenance, 1, 0, true, true);
        assert_eq!(
            action,
            CardAction::Single {
                label_key: "task_card.action.maintenance",
                style: ActionStyle::Danger
            }
        );
    }

    #[test]
    fn every_archetype_has_its_own_accent_class() {
        let mut seen = Vec::new();
        for archetype in ALL {
            let class = accent_class(archetype);
            assert!(class.starts_with("task-item--"), "{class} is not a modifier");
            assert!(!seen.contains(&class), "{class} used twice");
            seen.push(class);
        }
    }

    #[test]
    fn every_single_action_has_its_own_label() {
        let mut seen = Vec::new();
        for archetype in ALL {
            let target = if archetype == Archetype::Bonus { 0 } else { 1 };
            let CardAction::Single { label_key, .. } =
                card_action(archetype, target, 0, true, true)
            else {
                panic!("{archetype:?} did not yield a single button");
            };
            assert!(
                label_key.starts_with("task_card.action."),
                "{label_key} is not an action key"
            );
            assert!(!seen.contains(&label_key), "{label_key} used twice");
            seen.push(label_key);
        }
    }

    // ---- optimistic counter -------------------------------------------------

    #[test]
    fn pending_taps_are_added_to_the_server_count() {
        assert_eq!(effective_completions(2, 3), 5);
    }

    /// Tapping `−` more often than there are completions must not produce a negative display.
    #[test]
    fn the_displayed_count_never_goes_negative() {
        assert_eq!(effective_completions(1, -4), 0);
    }

    /// The core of the rework: the button has to close on the *optimistic* count. Judging by the
    /// server's count would leave `+` enabled through a whole burst and overshoot the target.
    #[test]
    fn plus_closes_once_the_pending_taps_reach_the_target() {
        // Server still says 0 of 3, but three taps are already queued.
        let effective = effective_completions(0, 3);
        assert!(!can_complete_pending(true, false, 3, effective));
    }

    #[test]
    fn plus_stays_open_below_the_target() {
        let effective = effective_completions(0, 2);
        assert!(can_complete_pending(true, false, 3, effective));
    }

    #[test]
    fn allow_exceed_keeps_plus_open_past_the_target() {
        let effective = effective_completions(3, 2);
        assert!(can_complete_pending(true, true, 3, effective));
    }

    /// A bonus task has no target, so `+` never closes.
    #[test]
    fn a_task_without_a_target_never_closes_plus() {
        let effective = effective_completions(0, 9);
        assert!(can_complete_pending(true, false, 0, effective));
    }

    #[test]
    fn permission_still_overrides_everything() {
        assert!(!can_complete_pending(false, true, 0, 0));
    }

    #[test]
    fn undo_needs_something_to_take_back() {
        assert!(!can_undo_pending(true, 0));
        assert!(can_undo_pending(true, 1));
    }

    /// Pending taps make undo available before the server has heard about them.
    #[test]
    fn undo_opens_on_a_pending_tap_alone() {
        let effective = effective_completions(0, 1);
        assert!(can_undo_pending(true, effective));
    }

    #[test]
    fn undo_stays_shut_without_the_permission() {
        assert!(!can_undo_pending(false, 5));
    }

    #[test]
    fn only_the_four_special_types_carry_a_badge() {
        assert_eq!(type_badge(Archetype::OneOff), None);
        assert_eq!(type_badge(Archetype::Routine), None);
        for archetype in [
            Archetype::Shared,
            Archetype::BadHabit,
            Archetype::Maintenance,
            Archetype::Bonus,
        ] {
            assert!(
                type_badge(archetype).is_some(),
                "{archetype:?} lost its badge"
            );
        }
    }
}
