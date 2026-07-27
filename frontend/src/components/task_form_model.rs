//! Signal-free form logic for the archetype-driven task form.
//!
//! The task form no longer asks the user to know that "maintenance" means
//! `habit_type: Bad` plus `assignee_cannot_uncomplete`. It asks *what* is being created and
//! derives the flags from that. Everything needed for that translation lives here as plain
//! data and pure functions — no Leptos, no signals, no DOM — so it runs under `#[test]` on
//! the host instead of only in a browser.
//!
//! The archetype is an entry point, never a cage: nothing in here disables, hides or locks a
//! field. It only decides what a form *starts* with and which groups start expanded.

use chrono::NaiveDate;
use shared::{Archetype, HabitType, RecurrenceType};

/// Date format of `<input type="date">`.
const DATE_INPUT_FORMAT: &str = "%Y-%m-%d";

/// Order of the type cards in create mode.
pub const ALL_ARCHETYPES: [Archetype; 5] = [
    Archetype::OneOff,
    Archetype::Routine,
    Archetype::Shared,
    Archetype::BadHabit,
    Archetype::Maintenance,
];

/// Visual weight of an archetype's explanatory note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteKind {
    /// Explains a consequence of the choice.
    Info,
    /// Warns about a restriction the user cannot undo from inside the task.
    Danger,
}

impl NoteKind {
    /// CSS modifier appended to `.task-form-note`.
    pub fn css_class(&self) -> &'static str {
        match self {
            NoteKind::Info => "task-form-note",
            NoteKind::Danger => "task-form-note danger",
        }
    }
}

/// What an archetype knows about itself inside the form.
///
/// All texts are i18n keys, never literals — the form is translated like everything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchetypePreset {
    pub icon: &'static str,
    pub name_key: &'static str,
    pub desc_key: &'static str,
    pub form_title_key: &'static str,
    /// Only three archetypes carry a note; `OneOff` and `Routine` need no explanation.
    pub note: Option<(NoteKind, &'static str)>,
    pub assign_label_key: &'static str,
    pub assign_hint_key: &'static str,
    /// `true` only for `OneOff`: its base field is a single date, not a recurrence selector.
    /// The recurrence selector stays reachable — it moves into the "details" group.
    pub base_is_date: bool,
}

/// The presentation preset behind an archetype.
pub fn preset(archetype: Archetype) -> ArchetypePreset {
    match archetype {
        Archetype::OneOff => ArchetypePreset {
            icon: "☑️",
            name_key: "task_modal.archetype.oneoff.name",
            desc_key: "task_modal.archetype.oneoff.desc",
            form_title_key: "task_modal.archetype.oneoff.form_title",
            note: None,
            assign_label_key: "task_modal.archetype.oneoff.assign_label",
            assign_hint_key: "task_modal.archetype.oneoff.assign_hint",
            base_is_date: true,
        },
        Archetype::Routine => ArchetypePreset {
            icon: "🔁",
            name_key: "task_modal.archetype.routine.name",
            desc_key: "task_modal.archetype.routine.desc",
            form_title_key: "task_modal.archetype.routine.form_title",
            note: None,
            assign_label_key: "task_modal.archetype.routine.assign_label",
            assign_hint_key: "task_modal.archetype.routine.assign_hint",
            base_is_date: false,
        },
        Archetype::Shared => ArchetypePreset {
            icon: "👥",
            name_key: "task_modal.archetype.shared.name",
            desc_key: "task_modal.archetype.shared.desc",
            form_title_key: "task_modal.archetype.shared.form_title",
            note: Some((NoteKind::Info, "task_modal.archetype.shared.note")),
            assign_label_key: "task_modal.archetype.shared.assign_label",
            assign_hint_key: "task_modal.archetype.shared.assign_hint",
            base_is_date: false,
        },
        Archetype::BadHabit => ArchetypePreset {
            icon: "⚠️",
            name_key: "task_modal.archetype.bad_habit.name",
            desc_key: "task_modal.archetype.bad_habit.desc",
            form_title_key: "task_modal.archetype.bad_habit.form_title",
            note: Some((NoteKind::Info, "task_modal.archetype.bad_habit.note")),
            assign_label_key: "task_modal.archetype.bad_habit.assign_label",
            assign_hint_key: "task_modal.archetype.bad_habit.assign_hint",
            base_is_date: false,
        },
        Archetype::Maintenance => ArchetypePreset {
            icon: "🛡️",
            name_key: "task_modal.archetype.maintenance.name",
            desc_key: "task_modal.archetype.maintenance.desc",
            form_title_key: "task_modal.archetype.maintenance.form_title",
            note: Some((NoteKind::Danger, "task_modal.archetype.maintenance.note")),
            assign_label_key: "task_modal.archetype.maintenance.assign_label",
            assign_hint_key: "task_modal.archetype.maintenance.assign_hint",
            base_is_date: false,
        },
    }
}

/// Snapshot of the switches that decide which archetype the form currently *is*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormFlags {
    pub habit_bad: bool,
    pub anyone_can_complete: bool,
    pub assignee_cannot_uncomplete: bool,
    pub recurrence: String,
}

/// Mirrors `Task::archetype()` and breaks the `OneOff`/`Routine` tie via the user's pick.
///
/// `Task::archetype()` cannot do that — it has no notion of a selection, so a "task with a
/// date" (which is a custom recurrence, see [`apply_onetime_date`]) would immediately read as
/// "Routine · changed". The form does know, so it uses that knowledge for exactly this one
/// tie. A recurrence of `onetime` always wins for `OneOff`, even when `Routine` was picked —
/// that keeps the chip in agreement with `Task::archetype()` and correctly flags the drift.
pub fn derive_archetype(flags: &FormFlags, selected: Archetype) -> Archetype {
    if flags.assignee_cannot_uncomplete {
        Archetype::Maintenance
    } else if flags.habit_bad {
        Archetype::BadHabit
    } else if flags.anyone_can_complete {
        Archetype::Shared
    } else if flags.recurrence == RecurrenceType::OneTime.as_str()
        || selected == Archetype::OneOff
    {
        // First half mirrors Task::archetype(), second half is the tie the form can break.
        Archetype::OneOff
    } else {
        Archetype::Routine
    }
}

/// Whether the form creates a new task or edits an existing one.
///
/// Duplicating counts as `Create`: there is no task to update yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormMode {
    Create,
    Edit,
}

/// Full snapshot for the "starts expanded" decision.
///
/// `on_dashboard` is deliberately absent: it is fetched asynchronously after mount, so at
/// decision time it is always `false` and could never open a group anyway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormSnapshot {
    pub description_empty: bool,
    pub category_set: bool,
    pub due_time_set: bool,
    pub recurrence: String,
    pub custom_dates_len: usize,
    pub target_count: String,
    pub allow_exceed: bool,
    pub habit_bad: bool,
    pub points_reward_set: bool,
    pub points_penalty_set: bool,
    pub linked_rewards: usize,
    pub linked_punishments: usize,
    pub anyone_can_complete: bool,
    pub assignee_cannot_uncomplete: bool,
    pub requires_review: bool,
}

/// Which of the four accordion groups start expanded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenGroups {
    pub details: bool,
    pub goal: bool,
    pub points: bool,
    pub rules: bool,
}

/// Whether the stored recurrence is what the archetype's preset would have produced.
///
/// For `OneOff` the preset is `OneTime`, but the date base field maps a single chosen day to
/// `Custom` + one date — so that shape counts as matching too. For every other archetype the
/// preset leaves the rhythm to the user; only `onetime` contradicts the archetype.
fn recurrence_matches_preset(recurrence: &str, custom_dates_len: usize, selected: Archetype) -> bool {
    match selected.defaults().recurrence_type {
        Some(RecurrenceType::OneTime) => {
            recurrence == RecurrenceType::OneTime.as_str()
                || (recurrence == RecurrenceType::Custom.as_str() && custom_dates_len == 1)
        }
        Some(other) => recurrence == other.as_str(),
        None => recurrence != RecurrenceType::OneTime.as_str(),
    }
}

/// A group starts expanded when it holds a value that deviates from the archetype's preset.
///
/// In create mode only preset-driven deviations count: household defaults (points, linked
/// rewards) are no deviation from the archetype and must not expand "points & consequences".
/// The rules group is the exception — it opens whenever any permission is set at all, in both
/// modes, because a granted permission is always worth seeing.
pub fn initial_open_groups(mode: FormMode, s: &FormSnapshot, selected: Archetype) -> OpenGroups {
    let preset_habit_bad = selected.defaults().habit_type == HabitType::Bad;
    let habit_deviates = s.habit_bad != preset_habit_bad;
    let recurrence_deviates =
        !recurrence_matches_preset(&s.recurrence, s.custom_dates_len, selected);

    let rules =
        s.anyone_can_complete || s.assignee_cannot_uncomplete || s.requires_review;

    match mode {
        FormMode::Create => OpenGroups {
            details: recurrence_deviates,
            goal: habit_deviates,
            points: false,
            rules,
        },
        FormMode::Edit => OpenGroups {
            details: recurrence_deviates
                || !s.description_empty
                || s.category_set
                || s.due_time_set,
            goal: habit_deviates || s.target_count.trim() != "1" || !s.allow_exceed,
            points: s.points_reward_set
                || s.points_penalty_set
                || s.linked_rewards > 0
                || s.linked_punishments > 0,
            rules,
        },
    }
}

/// Maintenance without a responsible person: `TaskWithStatus::is_assignee` requires
/// `assigned_user_id.is_some()`, so without one the `can_uncomplete` lock never engages and
/// the task would be silently ineffective.
pub fn assignment_missing(selected: Archetype, assigned_user: &str) -> bool {
    selected.assignment_required() && assigned_user.trim().is_empty()
}

/// Recurrence after switching type: a fixed preset wins, otherwise the user's own choice is
/// kept — unless it was `onetime`, which no recurring archetype can live with.
pub fn recurrence_after_preset(current: &str, selected: Archetype) -> String {
    match selected.defaults().recurrence_type {
        Some(fixed) => fixed.as_str().to_string(),
        None if current == RecurrenceType::OneTime.as_str() => {
            RecurrenceType::Daily.as_str().to_string()
        }
        None => current.to_string(),
    }
}

/// A bad habit is tracked by the person having it, so it is prefilled with the current user —
/// but only while nobody is assigned yet, and it stays freely changeable afterwards.
pub fn assignment_after_preset(
    selected: Archetype,
    current: &str,
    current_user_id: Option<&str>,
) -> String {
    if selected == Archetype::BadHabit && current.trim().is_empty() {
        current_user_id.unwrap_or_default().to_string()
    } else {
        current.to_string()
    }
}

/// Value for the `OneOff` date base field.
///
/// There is no dedicated date column on a task: a date *is* a custom recurrence holding
/// exactly that one day. Several days belong to the calendar picker, so the single-line date
/// field stays empty for them rather than silently dropping any.
pub fn onetime_date_value(recurrence: &str, custom_dates: &[NaiveDate]) -> String {
    if recurrence == RecurrenceType::Custom.as_str() && custom_dates.len() == 1 {
        custom_dates[0].format(DATE_INPUT_FORMAT).to_string()
    } else {
        String::new()
    }
}

/// Inverse of [`onetime_date_value`]: what the recurrence signals become when a date is
/// picked or cleared.
pub fn apply_onetime_date(date: Option<NaiveDate>) -> (&'static str, Vec<NaiveDate>) {
    match date {
        Some(day) => (RecurrenceType::Custom.as_str(), vec![day]),
        None => (RecurrenceType::OneTime.as_str(), Vec::new()),
    }
}

/// Whether the reward/punishment linking section is shown.
///
/// An existing task may still carry links from before the household switched the feature off.
/// Hiding them would make the form silently swallow data on the next save, so already linked
/// entries keep the section visible.
pub fn links_section_visible(enabled: bool, already_linked: usize) -> bool {
    enabled || already_linked > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{RecurrenceValue, Task};
    use uuid::Uuid;

    fn flags(habit_bad: bool, anyone: bool, no_uncomplete: bool, recurrence: &str) -> FormFlags {
        FormFlags {
            habit_bad,
            anyone_can_complete: anyone,
            assignee_cannot_uncomplete: no_uncomplete,
            recurrence: recurrence.to_string(),
        }
    }

    /// A snapshot that deviates from nothing — every test only flips what it is about.
    fn neutral(recurrence: &str) -> FormSnapshot {
        FormSnapshot {
            description_empty: true,
            category_set: false,
            due_time_set: false,
            recurrence: recurrence.to_string(),
            custom_dates_len: 0,
            target_count: "1".to_string(),
            allow_exceed: true,
            habit_bad: false,
            points_reward_set: false,
            points_penalty_set: false,
            linked_rewards: 0,
            linked_punishments: 0,
            anyone_can_complete: false,
            assignee_cannot_uncomplete: false,
            requires_review: false,
        }
    }

    /// Snapshot carrying exactly the preset of `archetype`, nothing else.
    fn preset_snapshot(archetype: Archetype) -> FormSnapshot {
        let defaults = archetype.defaults();
        let recurrence = defaults
            .recurrence_type
            .unwrap_or(RecurrenceType::Daily)
            .as_str();
        let mut snapshot = neutral(recurrence);
        snapshot.habit_bad = defaults.habit_type == HabitType::Bad;
        snapshot.anyone_can_complete = defaults.anyone_can_complete;
        snapshot.assignee_cannot_uncomplete = defaults.assignee_cannot_uncomplete;
        snapshot
    }

    fn task_with(
        habit_type: HabitType,
        anyone_can_complete: bool,
        assignee_cannot_uncomplete: bool,
        recurrence_type: RecurrenceType,
    ) -> Task {
        Task {
            id: Uuid::new_v4(),
            household_id: Uuid::new_v4(),
            title: "Test".to_string(),
            description: String::new(),
            recurrence_type,
            recurrence_value: None::<RecurrenceValue>,
            assigned_user_id: None,
            target_count: 1,
            time_period: None,
            allow_exceed_target: true,
            anyone_can_complete,
            assignee_cannot_uncomplete,
            requires_review: false,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type,
            category_id: None,
            category_name: None,
            archived: false,
            paused: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            suggestion: None,
            suggested_by: None,
        }
    }

    // ---- derive_archetype -------------------------------------------------

    #[test]
    fn no_uncomplete_always_derives_maintenance() {
        // Even a good, shared, one-time task reads as maintenance once the lock is on:
        // not being able to undo your own completion shapes the interaction most.
        let f = flags(false, true, true, "onetime");
        assert_eq!(derive_archetype(&f, Archetype::OneOff), Archetype::Maintenance);
    }

    #[test]
    fn bad_habit_wins_over_anyone_can_complete() {
        let f = flags(true, true, false, "daily");
        assert_eq!(derive_archetype(&f, Archetype::Shared), Archetype::BadHabit);
    }

    #[test]
    fn anyone_can_complete_with_good_habit_is_shared() {
        let f = flags(false, true, false, "weekly");
        assert_eq!(derive_archetype(&f, Archetype::Routine), Archetype::Shared);
    }

    #[test]
    fn onetime_recurrence_overrides_a_routine_selection() {
        // Matches Task::archetype(): a one-time task is a one-off, whatever was picked.
        let f = flags(false, false, false, "onetime");
        assert_eq!(derive_archetype(&f, Archetype::Routine), Archetype::OneOff);
    }

    #[test]
    fn oneoff_selection_survives_a_picked_date() {
        // A date is stored as a custom recurrence; without the selection tiebreak the chip
        // would immediately read "Routine · changed".
        let f = flags(false, false, false, "custom");
        assert_eq!(derive_archetype(&f, Archetype::OneOff), Archetype::OneOff);
    }

    #[test]
    fn routine_selection_with_daily_recurrence_stays_routine() {
        let f = flags(false, false, false, "daily");
        assert_eq!(derive_archetype(&f, Archetype::Routine), Archetype::Routine);
    }

    #[test]
    fn derives_the_same_archetype_as_task_archetype_for_all_five() {
        for archetype in ALL_ARCHETYPES {
            let defaults = archetype.defaults();
            let recurrence = defaults.recurrence_type.unwrap_or(RecurrenceType::Daily);
            let recurrence_str = recurrence.as_str();
            let habit_bad = defaults.habit_type == HabitType::Bad;
            let task = task_with(
                defaults.habit_type,
                defaults.anyone_can_complete,
                defaults.assignee_cannot_uncomplete,
                recurrence,
            );
            let f = flags(
                habit_bad,
                defaults.anyone_can_complete,
                defaults.assignee_cannot_uncomplete,
                recurrence_str,
            );
            assert_eq!(task.archetype(), archetype, "Task::archetype for {archetype:?}");
            assert_eq!(
                derive_archetype(&f, archetype),
                task.archetype(),
                "derive_archetype disagrees with Task::archetype for {archetype:?}"
            );
        }
    }

    // ---- initial_open_groups: create mode ---------------------------------

    #[test]
    fn create_oneoff_starts_with_every_group_closed() {
        let groups = initial_open_groups(
            FormMode::Create,
            &preset_snapshot(Archetype::OneOff),
            Archetype::OneOff,
        );
        assert_eq!(
            groups,
            OpenGroups { details: false, goal: false, points: false, rules: false }
        );
    }

    #[test]
    fn create_routine_starts_with_every_group_closed() {
        let groups = initial_open_groups(
            FormMode::Create,
            &preset_snapshot(Archetype::Routine),
            Archetype::Routine,
        );
        assert_eq!(
            groups,
            OpenGroups { details: false, goal: false, points: false, rules: false }
        );
    }

    #[test]
    fn create_shared_opens_only_the_rules_group() {
        let groups = initial_open_groups(
            FormMode::Create,
            &preset_snapshot(Archetype::Shared),
            Archetype::Shared,
        );
        assert_eq!(
            groups,
            OpenGroups { details: false, goal: false, points: false, rules: true }
        );
    }

    #[test]
    fn create_maintenance_opens_only_the_rules_group() {
        // habit_bad matches the maintenance preset, so "goal" stays closed.
        let groups = initial_open_groups(
            FormMode::Create,
            &preset_snapshot(Archetype::Maintenance),
            Archetype::Maintenance,
        );
        assert_eq!(
            groups,
            OpenGroups { details: false, goal: false, points: false, rules: true }
        );
    }

    #[test]
    fn create_bad_habit_opens_no_group() {
        let groups = initial_open_groups(
            FormMode::Create,
            &preset_snapshot(Archetype::BadHabit),
            Archetype::BadHabit,
        );
        assert_eq!(
            groups,
            OpenGroups { details: false, goal: false, points: false, rules: false }
        );
    }

    #[test]
    fn create_keeps_points_closed_despite_household_defaults() {
        // Household defaults are no deviation from the archetype.
        let mut snapshot = preset_snapshot(Archetype::Routine);
        snapshot.points_reward_set = true;
        snapshot.points_penalty_set = true;
        snapshot.linked_rewards = 2;
        let groups = initial_open_groups(FormMode::Create, &snapshot, Archetype::Routine);
        assert!(!groups.points);
    }

    // ---- initial_open_groups: edit mode -----------------------------------

    #[test]
    fn edit_description_opens_details() {
        let mut snapshot = neutral("daily");
        snapshot.description_empty = false;
        let groups = initial_open_groups(FormMode::Edit, &snapshot, Archetype::Routine);
        assert!(groups.details);
    }

    #[test]
    fn edit_due_time_opens_details() {
        let mut snapshot = neutral("daily");
        snapshot.due_time_set = true;
        let groups = initial_open_groups(FormMode::Edit, &snapshot, Archetype::Routine);
        assert!(groups.details);
    }

    #[test]
    fn edit_category_opens_details() {
        let mut snapshot = neutral("daily");
        snapshot.category_set = true;
        let groups = initial_open_groups(FormMode::Edit, &snapshot, Archetype::Routine);
        assert!(groups.details);
    }

    #[test]
    fn edit_target_count_above_one_opens_goal() {
        let mut snapshot = neutral("daily");
        snapshot.target_count = "3".to_string();
        let groups = initial_open_groups(FormMode::Edit, &snapshot, Archetype::Routine);
        assert!(groups.goal);
    }

    #[test]
    fn edit_disallowed_exceed_opens_goal() {
        let mut snapshot = neutral("daily");
        snapshot.allow_exceed = false;
        let groups = initial_open_groups(FormMode::Edit, &snapshot, Archetype::Routine);
        assert!(groups.goal);
    }

    #[test]
    fn edit_bad_habit_on_bad_habit_archetype_keeps_goal_closed() {
        // The habit type is what makes it a bad habit — no deviation, nothing to reveal.
        let mut snapshot = neutral("daily");
        snapshot.habit_bad = true;
        let groups = initial_open_groups(FormMode::Edit, &snapshot, Archetype::BadHabit);
        assert!(!groups.goal);
    }

    #[test]
    fn edit_bad_habit_on_routine_opens_goal() {
        let mut snapshot = neutral("daily");
        snapshot.habit_bad = true;
        let groups = initial_open_groups(FormMode::Edit, &snapshot, Archetype::Routine);
        assert!(groups.goal);
    }

    #[test]
    fn edit_points_reward_opens_points() {
        let mut snapshot = neutral("daily");
        snapshot.points_reward_set = true;
        let groups = initial_open_groups(FormMode::Edit, &snapshot, Archetype::Routine);
        assert!(groups.points);
    }

    #[test]
    fn edit_linked_reward_opens_points() {
        let mut snapshot = neutral("daily");
        snapshot.linked_rewards = 1;
        let groups = initial_open_groups(FormMode::Edit, &snapshot, Archetype::Routine);
        assert!(groups.points);
    }

    #[test]
    fn edit_requires_review_opens_rules() {
        let mut snapshot = neutral("daily");
        snapshot.requires_review = true;
        let groups = initial_open_groups(FormMode::Edit, &snapshot, Archetype::Routine);
        assert!(groups.rules);
    }

    #[test]
    fn edit_oneoff_with_a_single_date_keeps_details_closed() {
        // The date is shown by the base field, so nothing is hidden inside "details".
        let mut snapshot = neutral("custom");
        snapshot.custom_dates_len = 1;
        let groups = initial_open_groups(FormMode::Edit, &snapshot, Archetype::OneOff);
        assert!(!groups.details);
    }

    #[test]
    fn edit_oneoff_with_weekly_recurrence_opens_details() {
        // The recurrence selector lives in "details" for one-offs — it must be visible.
        let snapshot = neutral("weekly");
        let groups = initial_open_groups(FormMode::Edit, &snapshot, Archetype::OneOff);
        assert!(groups.details);
    }

    #[test]
    fn edit_oneoff_with_several_dates_opens_details() {
        // Several dates belong to the calendar picker, which sits in "details".
        let mut snapshot = neutral("custom");
        snapshot.custom_dates_len = 3;
        let groups = initial_open_groups(FormMode::Edit, &snapshot, Archetype::OneOff);
        assert!(groups.details);
    }

    #[test]
    fn edit_routine_with_onetime_recurrence_opens_details() {
        let snapshot = neutral("onetime");
        let groups = initial_open_groups(FormMode::Edit, &snapshot, Archetype::Routine);
        assert!(groups.details);
    }

    // ---- assignment_missing -----------------------------------------------

    #[test]
    fn maintenance_without_assignment_is_missing() {
        assert!(assignment_missing(Archetype::Maintenance, ""));
    }

    #[test]
    fn maintenance_with_assignment_is_complete() {
        assert!(!assignment_missing(
            Archetype::Maintenance,
            "550e8400-e29b-41d4-a716-446655440000"
        ));
    }

    #[test]
    fn other_archetypes_never_require_an_assignment() {
        for archetype in ALL_ARCHETYPES {
            if archetype == Archetype::Maintenance {
                continue;
            }
            assert!(
                !assignment_missing(archetype, ""),
                "{archetype:?} must not require an assignment"
            );
        }
    }

    // ---- recurrence_after_preset ------------------------------------------

    #[test]
    fn oneoff_always_forces_onetime_recurrence() {
        assert_eq!(recurrence_after_preset("weekly", Archetype::OneOff), "onetime");
        assert_eq!(recurrence_after_preset("custom", Archetype::OneOff), "onetime");
    }

    #[test]
    fn routine_replaces_onetime_with_daily() {
        assert_eq!(recurrence_after_preset("onetime", Archetype::Routine), "daily");
    }

    #[test]
    fn routine_keeps_an_existing_rhythm() {
        assert_eq!(recurrence_after_preset("weekly", Archetype::Routine), "weekly");
    }

    // ---- assignment_after_preset ------------------------------------------

    #[test]
    fn bad_habit_prefills_the_current_user() {
        assert_eq!(
            assignment_after_preset(Archetype::BadHabit, "", Some("user-1")),
            "user-1"
        );
    }

    #[test]
    fn bad_habit_keeps_an_existing_assignment() {
        assert_eq!(
            assignment_after_preset(Archetype::BadHabit, "user-2", Some("user-1")),
            "user-2"
        );
    }

    #[test]
    fn bad_habit_without_current_user_stays_empty() {
        assert_eq!(assignment_after_preset(Archetype::BadHabit, "", None), "");
    }

    #[test]
    fn other_archetypes_do_not_prefill_the_assignment() {
        for archetype in ALL_ARCHETYPES {
            if archetype == Archetype::BadHabit {
                continue;
            }
            assert_eq!(
                assignment_after_preset(archetype, "", Some("user-1")),
                "",
                "{archetype:?} must not prefill the assignment"
            );
        }
    }

    // ---- date mapping ------------------------------------------------------

    #[test]
    fn onetime_recurrence_has_no_date_value() {
        assert_eq!(onetime_date_value("onetime", &[]), "");
    }

    #[test]
    fn a_single_custom_date_is_the_date_value() {
        let day = NaiveDate::from_ymd_opt(2026, 8, 1).expect("valid date");
        assert_eq!(onetime_date_value("custom", &[day]), "2026-08-01");
    }

    #[test]
    fn several_custom_dates_leave_the_date_field_empty() {
        let first = NaiveDate::from_ymd_opt(2026, 8, 1).expect("valid date");
        let second = NaiveDate::from_ymd_opt(2026, 8, 2).expect("valid date");
        assert_eq!(onetime_date_value("custom", &[first, second]), "");
    }

    #[test]
    fn clearing_the_date_falls_back_to_onetime() {
        assert_eq!(apply_onetime_date(None), ("onetime", Vec::new()));
    }

    #[test]
    fn picking_a_date_stores_it_as_a_single_custom_date() {
        let day = NaiveDate::from_ymd_opt(2026, 8, 1).expect("valid date");
        assert_eq!(apply_onetime_date(Some(day)), ("custom", vec![day]));
    }

    #[test]
    fn date_value_and_apply_are_inverse() {
        let day = NaiveDate::from_ymd_opt(2026, 12, 24).expect("valid date");
        let (recurrence, dates) = apply_onetime_date(Some(day));
        assert_eq!(onetime_date_value(recurrence, &dates), "2026-12-24");
    }

    // ---- links_section_visible --------------------------------------------

    #[test]
    fn links_section_hidden_when_disabled_and_unused() {
        assert!(!links_section_visible(false, 0));
    }

    #[test]
    fn links_section_visible_for_existing_links_despite_disabled_feature() {
        assert!(links_section_visible(false, 1));
    }

    #[test]
    fn links_section_visible_when_enabled() {
        assert!(links_section_visible(true, 0));
    }

    // ---- presets -----------------------------------------------------------

    #[test]
    fn only_oneoff_uses_a_date_as_its_base_field() {
        for archetype in ALL_ARCHETYPES {
            assert_eq!(
                preset(archetype).base_is_date,
                archetype == Archetype::OneOff,
                "base_is_date wrong for {archetype:?}"
            );
        }
    }

    #[test]
    fn only_three_archetypes_carry_a_note() {
        assert!(preset(Archetype::OneOff).note.is_none());
        assert!(preset(Archetype::Routine).note.is_none());
        assert_eq!(
            preset(Archetype::Shared).note.map(|(kind, _)| kind),
            Some(NoteKind::Info)
        );
        assert_eq!(
            preset(Archetype::BadHabit).note.map(|(kind, _)| kind),
            Some(NoteKind::Info)
        );
        assert_eq!(
            preset(Archetype::Maintenance).note.map(|(kind, _)| kind),
            Some(NoteKind::Danger)
        );
    }

    #[test]
    fn every_archetype_has_its_own_i18n_keys() {
        let mut seen = Vec::new();
        for archetype in ALL_ARCHETYPES {
            let p = preset(archetype);
            for key in [p.name_key, p.desc_key, p.form_title_key, p.assign_label_key, p.assign_hint_key] {
                assert!(key.starts_with("task_modal.archetype."), "{key} is not an archetype key");
                assert!(!seen.contains(&key), "{key} used twice");
                seen.push(key);
            }
        }
        assert_eq!(seen.len(), 25);
    }
}
