# Gamification System Flow

## Points System Overview

```mermaid
flowchart TB
    subgraph "Points Sources"
        TaskComplete[Task Completed<br/>+points_reward]
        TaskMiss[Task Missed<br/>-points_penalty]
        Streak[Streak Bonus<br/>+streak points]
        Manual[Manual Adjustment<br/>±points]
        Purchase[Reward Purchase<br/>-point_cost]
    end

    subgraph "Points Storage"
        Membership[household_memberships.points]
    end

    TaskComplete & TaskMiss & Streak & Manual & Purchase --> Membership
```

## Reward Flow

```mermaid
flowchart TB
    subgraph "Reward Acquisition"
        TaskLink[Linked to Task<br/>Auto-assign on complete]
        Purchase[Purchase<br/>Spend points]
        AdminAssign[Admin Assignment<br/>Direct grant]
    end

    subgraph "User Reward State"
        UR[user_rewards]
        Amount[amount: total assigned]
        Redeemed[redeemed_amount: used]
        Pending[pending_redemption: awaiting approval]
    end

    subgraph "Redemption"
        Redeem[User redeems]
        Check{requires_confirmation?}
        AutoApprove[Instant approval]
        PendingApproval[Wait for admin]
        Approve[Admin approves]
        Reject[Admin rejects]
    end

    TaskLink & Purchase & AdminAssign --> UR
    UR --> Amount & Redeemed & Pending

    Redeem --> Check
    Check -->|No| AutoApprove
    Check -->|Yes| PendingApproval
    PendingApproval --> Approve
    PendingApproval --> Reject
```

## Punishment Flow

```mermaid
flowchart TB
    subgraph "Punishment Assignment"
        TaskMiss[Task Missed<br/>Auto-assign]
        AdminAssign[Admin Assignment<br/>Direct assign]
    end

    subgraph "User Punishment State"
        UP[user_punishments]
        Amount[amount: total assigned]
        Completed[completed_amount: done]
        Pending[pending_completion: awaiting approval]
    end

    subgraph "Completion"
        Complete[User completes]
        Check{requires_confirmation?}
        AutoApprove[Instant approval]
        PendingApproval[Wait for admin]
        Approve[Admin approves]
        Reject[Admin rejects]
    end

    TaskMiss & AdminAssign --> UP
    UP --> Amount & Completed & Pending

    Complete --> Check
    Check -->|No| AutoApprove
    Check -->|Yes| PendingApproval
    PendingApproval --> Approve
    PendingApproval --> Reject
```

## Random Choice Punishment Flow

```mermaid
flowchart TB
    subgraph "Assignment"
        Assign[Random Choice Punishment<br/>Assigned to User]
    end

    subgraph "User Action"
        View[User views punishment]
        Pick[User clicks 'Pick one']
    end

    subgraph "Random Selection"
        GetOptions[Get linked punishment options]
        Random[System randomly selects one]
        CheckNested{Selected is<br/>random choice?}
    end

    subgraph "Resolution"
        AssignSelected[Assign selected punishment<br/>to user]
        MarkResolved[Mark original assignment<br/>as resolved]
        LogActivity[Log activity:<br/>PunishmentRandomPicked]
        PickAgain[User must pick again<br/>from nested options]
    end

    Assign --> View
    View --> Pick
    Pick --> GetOptions
    GetOptions --> Random
    Random --> CheckNested
    CheckNested -->|No| AssignSelected
    CheckNested -->|Yes| PickAgain
    PickAgain --> View
    AssignSelected --> MarkResolved
    MarkResolved --> LogActivity
```

## Habit Types

```mermaid
flowchart TB
    subgraph "Good Habit"
        GH[Good Habit Task]
        GComplete[Completed]
        GMiss[Missed]

        GH --> GComplete & GMiss
        GComplete -->|+points| GReward[Rewards applied]
        GMiss -->|-points| GPunish[Punishments applied]
    end

    subgraph "Bad Habit"
        BH[Bad Habit Task]
        BComplete[Completed<br/>Indulged]
        BMiss[Missed<br/>Resisted]

        BH --> BComplete & BMiss
        BComplete -->|-points| BPunish[Punishments applied]
        BMiss -->|+points| BReward[Rewards applied]
    end
```

## Streak Calculation

```mermaid
flowchart TB
    Check[Check task completion] --> GetHistory[Get completion history]
    GetHistory --> CalcStreak[Calculate consecutive completions]

    CalcStreak --> Compare{Compare with previous}

    Compare -->|Increased| UpdateStreak[Update streak count]
    Compare -->|Broken| ResetStreak[Reset streak to 0]
    Compare -->|Same| KeepStreak[Keep current streak]

    UpdateStreak --> CheckBonus{Streak >= threshold?}
    CheckBonus -->|Yes| ApplyBonus[Apply streak bonus points]
    CheckBonus -->|No| NoBonus[No bonus]

    ResetStreak --> CheckPenalty{StreakBroken condition?}
    CheckPenalty -->|Yes| ApplyPenalty[Apply penalty points]
    CheckPenalty -->|No| NoPenalty[No penalty]
```

## Point Conditions

```mermaid
flowchart TB
    Event[Event Occurs] --> FindCond[Find matching conditions]

    FindCond --> CondType{condition_type}

    CondType -->|task_complete| TC[Task was completed]
    CondType -->|task_missed| TM[Task was missed]
    CondType -->|streak| ST[Streak achieved]
    CondType -->|streak_broken| SB[Streak was broken]

    TC & TM & ST & SB --> CheckTask{task_id specified?}

    CheckTask -->|Yes| MatchTask{Matches this task?}
    CheckTask -->|No| ApplyGlobal[Apply to all tasks]

    MatchTask -->|Yes| ApplySpecific[Apply to this task]
    MatchTask -->|No| Skip[Skip condition]

    ApplyGlobal & ApplySpecific --> CheckThreshold{streak_threshold?}

    CheckThreshold -->|Yes| MeetsThreshold{streak >= threshold?}
    CheckThreshold -->|No| CalcPoints[Calculate points]

    MeetsThreshold -->|Yes| CalcPoints
    MeetsThreshold -->|No| Skip2[Skip - threshold not met]

    CalcPoints --> CheckMult{multiplier?}
    CheckMult -->|Yes| ApplyMult[points * multiplier]
    CheckMult -->|No| BasePoints[Use base points]

    ApplyMult & BasePoints --> UpdateMember[Update member points]
```

## Leaderboard Calculation

```mermaid
flowchart TB
    Request[Get Leaderboard] --> QueryMembers[Query household_memberships]

    QueryMembers --> ForEach[For each member]

    ForEach --> GetPoints[Get points]
    ForEach --> CountTasks[Count approved completions]
    ForEach --> CalcStreak[Calculate current streak]

    GetPoints & CountTasks & CalcStreak --> BuildEntry[Build LeaderboardEntry]

    BuildEntry --> SortByPoints[Sort by points DESC]
    SortByPoints --> AssignRank[Assign rank positions]
    AssignRank --> Return[Return leaderboard]
```

## Complete Task Gamification Flow

```mermaid
sequenceDiagram
    participant U as User
    participant T as TaskService
    participant P as PointService
    participant R as RewardService
    participant A as ActivityService
    participant DB as Database

    U->>T: Complete Task
    T->>DB: Create completion record

    T->>P: Calculate base points
    P->>DB: Get point_conditions
    P->>P: Apply conditions & multipliers
    P->>DB: Update membership.points

    T->>R: Apply task rewards
    R->>DB: Get task_rewards
    loop Each linked reward
        R->>DB: Create/update user_reward
    end

    T->>T: Update streak
    T->>P: Check streak bonuses
    P->>DB: Apply streak points if applicable

    T->>A: Log TaskCompleted activity
    A->>DB: Create activity_log

    T-->>U: Completion result
```
