# Task Management Flow

## Task Completion Flow

```mermaid
sequenceDiagram
    participant U as User
    participant FE as Frontend
    participant BE as Backend
    participant DB as Database

    U->>FE: Complete task
    FE->>BE: POST /api/households/{id}/tasks/{task_id}/complete
    BE->>BE: Validate user can complete
    BE->>DB: Check completion count for period
    DB-->>BE: Current count

    alt Target reached & !allow_exceed
        BE-->>FE: 400 Target already reached
        FE-->>U: Show error
    else Can complete
        BE->>DB: Create task_completion
        alt requires_review = true
            Note right of BE: status = 'pending'
            BE-->>FE: Completion pending review
        else requires_review = false
            Note right of BE: status = 'approved'
            BE->>BE: Calculate points
            BE->>DB: Update member points
            BE->>DB: Apply linked rewards
            BE->>DB: Update streak
            BE->>DB: Log activity
            BE-->>FE: Task completed
        end
        FE-->>U: Show success
    end
```

## Task Review Flow

```mermaid
sequenceDiagram
    participant A as Admin/Owner
    participant FE as Frontend
    participant BE as Backend
    participant DB as Database

    A->>FE: View pending reviews
    FE->>BE: GET /api/households/{id}/tasks/pending
    BE->>DB: Query pending completions
    DB-->>BE: Pending list
    BE-->>FE: Pending completions
    FE-->>A: Display list

    A->>FE: Approve completion
    FE->>BE: POST .../completions/{id}/approve
    BE->>DB: Update status to 'approved'
    BE->>BE: Calculate points
    BE->>DB: Update member points
    BE->>DB: Apply linked rewards
    BE->>DB: Log activity
    BE-->>FE: Approved
    FE-->>A: Show success
```

## Points Calculation Flow

```mermaid
flowchart TB
    Complete[Task Completed] --> Type{Habit Type?}

    Type -->|Good| GoodCalc[Base Points = points_reward]
    Type -->|Bad| BadCalc[Base Points = -points_penalty]

    GoodCalc --> CheckCond[Check Point Conditions]
    BadCalc --> CheckCond

    CheckCond --> CondType{Condition Type?}

    CondType -->|task_complete| TCPoints[Add condition points]
    CondType -->|streak| StreakCheck{Streak >= threshold?}
    StreakCheck -->|Yes| StreakPoints[Add streak bonus]
    StreakCheck -->|No| NoBonus[No bonus]

    TCPoints --> Multiply{Has multiplier?}
    StreakPoints --> Multiply
    NoBonus --> Multiply

    Multiply -->|Yes| ApplyMult[Points * multiplier]
    Multiply -->|No| Final[Final Points]
    ApplyMult --> Final

    Final --> Update[Update member.points]
```

## Reward Assignment Flow

```mermaid
sequenceDiagram
    participant BE as Backend
    participant DB as Database

    Note over BE: Task completion approved

    BE->>DB: Get task_rewards for task
    DB-->>BE: Linked rewards list

    loop For each linked reward
        BE->>DB: Find/create user_reward
        alt Exists
            BE->>DB: Increment amount
        else New
            BE->>DB: Create user_reward
        end
        BE->>DB: Log RewardAssigned activity
    end
```

## Missed Task Processing

```mermaid
flowchart TB
    Scheduler[Scheduled Job] --> FindMissed[Find overdue tasks]
    FindMissed --> Loop{For each task}

    Loop --> CheckDue[Check if due date passed]
    CheckDue --> Completed{Was completed?}

    Completed -->|Yes| Skip[Skip - already done]
    Completed -->|No| CheckPenalty{Penalty tracked?}

    CheckPenalty -->|Yes| Skip2[Skip - already processed]
    CheckPenalty -->|No| Process[Process penalty]

    Process --> HabitType{Habit Type?}

    HabitType -->|Good| DeductPoints[Deduct points_penalty]
    HabitType -->|Bad| AddPoints[Add points_reward]

    DeductPoints --> ApplyPunish[Apply linked punishments]
    AddPoints --> ApplyReward[Apply linked rewards]

    ApplyPunish --> LogMissed[Log TaskMissed activity]
    ApplyReward --> LogMissed

    LogMissed --> Record[Record in missed_task_penalties]
    Record --> Loop
```

## Task Status Calculation

```mermaid
flowchart TB
    Task[Task] --> GetCompletions[Get completions in period]
    GetCompletions --> CalcStatus[Calculate status]

    CalcStatus --> Fields{Compute fields}

    Fields --> CC[completion_count]
    Fields --> RC[remaining_count<br/>target - completed]
    Fields --> Streak[current_streak]
    Fields --> LastComp[last_completion]
    Fields --> NextDue[next_due_date]
    Fields --> CanComp[can_complete]

    CanComp --> Check1{remaining > 0?}
    Check1 -->|Yes| True1[true]
    Check1 -->|No| Check2{allow_exceed?}
    Check2 -->|Yes| True2[true]
    Check2 -->|No| False[false]
```

## Task Detail Retrieval Flow

```mermaid
sequenceDiagram
    participant U as User
    participant FE as Frontend
    participant BE as Backend
    participant DB as Database

    U->>FE: Click task title
    FE->>FE: Open TaskDetailModal
    FE->>BE: GET /api/households/{id}/tasks/{task_id}/details

    BE->>DB: Get task by ID
    DB-->>BE: Task data

    BE->>DB: Get all completions for task
    DB-->>BE: Completion history

    BE->>BE: Calculate statistics
    Note right of BE: completion_rate (week/month/all-time)<br/>best_streak<br/>total_completions

    BE->>DB: Get assigned user details
    DB-->>BE: User data (if assigned)

    BE->>DB: Get linked rewards
    DB-->>BE: TaskRewardLinks

    BE->>DB: Get linked punishments
    DB-->>BE: TaskPunishmentLinks

    BE-->>FE: TaskWithDetails
    FE-->>U: Display detail modal
```

## Completion Rate Calculation

```mermaid
flowchart TB
    Start[Calculate Completion Rate] --> GetPeriod{Time Period?}

    GetPeriod -->|Week| WeekBounds[Get current week bounds]
    GetPeriod -->|Month| MonthBounds[Get current month bounds]
    GetPeriod -->|All Time| AllBounds[From task creation to now]

    WeekBounds & MonthBounds & AllBounds --> GetOccur[Count applicable occurrences<br/>based on recurrence pattern]

    GetOccur --> GetComp[Count completions where<br/>target was met]

    GetComp --> Calc[Rate = met / total × 100]

    Calc --> Result[Return rate + breakdown]
```

## Recurrence Logic

```mermaid
flowchart TB
    Task[Task] --> RecType{recurrence_type}

    RecType -->|daily| Daily[Every day]
    RecType -->|weekly| Weekly[Specific day of week<br/>recurrence_value = 0-6]
    RecType -->|monthly| Monthly[Specific day of month<br/>recurrence_value = 1-31]
    RecType -->|weekdays| Weekdays[Multiple days<br/>recurrence_value = JSON array]
    RecType -->|custom| Custom[Specific dates<br/>recurrence_value = date list]
    RecType -->|onetime| OneTime[Single occurrence]
    RecType -->|none| NoRecur[No schedule]

    Daily & Weekly & Monthly & Weekdays & Custom & OneTime & NoRecur --> Period{time_period}

    Period -->|day| PDay[Reset daily]
    Period -->|week| PWeek[Reset weekly]
    Period -->|month| PMonth[Reset monthly]
    Period -->|year| PYear[Reset yearly]
    Period -->|none| PNone[No reset]
```
