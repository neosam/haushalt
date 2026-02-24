# Roles and Permissions

## Role Hierarchy

```mermaid
flowchart TB
    Owner[Owner<br/>Full control]
    Admin[Admin<br/>Management]
    Member[Member<br/>Basic access]

    Owner -->|inherits| Admin
    Admin -->|inherits| Member
```

## Permission Matrix

```mermaid
flowchart LR
    subgraph Roles
        O[Owner]
        A[Admin]
        M[Member]
    end

    subgraph "Household Management"
        HU[Update household]
        HD[Delete household]
        HS[Update settings]
    end

    subgraph "Member Management"
        MV[View members]
        MR[Change roles]
        MRM[Remove members]
        MP[Adjust points]
        MI[Invite users]
    end

    subgraph "Task Management"
        TV[View tasks]
        TC[Create tasks]
        TU[Update tasks]
        TD[Delete tasks]
        TCO[Complete tasks]
        TR[Review completions]
    end

    subgraph "Rewards/Punishments"
        RV[View]
        RC[Create]
        RA[Assign]
        RR[Redeem]
        RPR[Purchase]
        RAP[Approve]
    end

    O --> HU & HD & HS
    O & A --> MV & MRM & MP & MI
    O --> MR
    O & A & M --> TV & TCO
    O & A --> TC & TU & TD & TR
    O & A & M --> RV & RR & RPR
    O & A --> RC & RA & RAP
```

## Hierarchy Types

```mermaid
flowchart TB
    subgraph "Equals Mode"
        E_O[Owner]
        E_A[Admin]
        E_M[Member]
        E_Task[Manage Tasks]
        E_Assign[Be Assigned]

        E_O & E_A & E_M --> E_Task
        E_O & E_A & E_M --> E_Assign
    end

    subgraph "Organized Mode"
        O_O[Owner]
        O_A[Admin]
        O_M[Member]
        O_Task[Manage Tasks]
        O_Assign[Be Assigned]

        O_O & O_A --> O_Task
        O_O & O_A & O_M --> O_Assign
    end

    subgraph "Hierarchy Mode"
        H_O[Owner]
        H_A[Admin]
        H_M[Member]
        H_Task[Manage Tasks]
        H_Assign[Be Assigned]

        H_O & H_A --> H_Task
        H_M --> H_Assign
    end
```

## Permission Check Flow

```mermaid
flowchart TB
    Action[Action Requested] --> GetRole[Get user role in household]
    GetRole --> CheckHierarchy[Check hierarchy_type setting]

    CheckHierarchy --> HType{Hierarchy Type}

    HType -->|Equals| EqualsCheck{Action type?}
    HType -->|Organized| OrgCheck{Action type?}
    HType -->|Hierarchy| HierCheck{Action type?}

    EqualsCheck -->|Manage| AllowAll[Allow for all roles]
    EqualsCheck -->|Assign| AllowAll2[Allow for all roles]

    OrgCheck -->|Manage| CheckAdmin{Is Admin+?}
    OrgCheck -->|Assign| AllowAll3[Allow for all roles]

    HierCheck -->|Manage| CheckAdmin2{Is Admin+?}
    HierCheck -->|Assign| CheckMember{Is Member only?}

    CheckAdmin -->|Yes| Allow[Allow]
    CheckAdmin -->|No| Deny[Deny]

    CheckAdmin2 -->|Yes| Allow2[Allow]
    CheckAdmin2 -->|No| Deny2[Deny]

    CheckMember -->|Yes| Allow3[Allow]
    CheckMember -->|No| Deny3[Deny]
```

## Activity Visibility

```mermaid
flowchart TB
    subgraph "Activity Log Access"
        Owner[Owner] --> AllLogs[See all activities]
        Admin[Admin] --> OwnLogs[See own activities only]
        Member[Member] --> OwnLogs2[See own activities only]
    end
```

## Invitation Permissions

```mermaid
flowchart LR
    subgraph "Can Invite"
        O[Owner]
        A[Admin]
    end

    subgraph "Roles to Assign"
        RM[Member]
        RA[Admin]
    end

    subgraph "Cannot Invite"
        M[Member]
    end

    O & A -->|can invite as| RM & RA
    M -.->|no permission| RM & RA
```

## Resource Ownership Rules

```mermaid
flowchart TB
    subgraph "Notes"
        NOwner[Note Author] -->|can| NEdit[Edit]
        NOwner -->|can| NDelete[Delete]
        NOther[Other Members] -->|can view if| NShared[is_shared = true]
    end

    subgraph "Chat Messages"
        CMOwner[Message Author] -->|can| CMEdit[Edit]
        CMOwner -->|can| CMDelete[Delete]
        CMAdmin[Admin/Owner] -->|can| CMDelete2[Delete any]
    end

    subgraph "Tasks"
        TAssignee[Assigned User] -->|can| TComplete[Complete]
        TAdmin[Admin+] -->|can| TManage[Create/Edit/Delete]
        TAdmin -->|can| TReview[Approve/Reject]
    end
```

## Feature Flags Impact

```mermaid
flowchart TB
    Settings[Household Settings]

    Settings --> RE{rewards_enabled}
    Settings --> PE{punishments_enabled}
    Settings --> CE{chat_enabled}

    RE -->|true| RewardsAvail[Rewards endpoints available]
    RE -->|false| RewardsHide[Rewards hidden/disabled]

    PE -->|true| PunishAvail[Punishments endpoints available]
    PE -->|false| PunishHide[Punishments hidden/disabled]

    CE -->|true| ChatAvail[Chat endpoints available]
    CE -->|false| ChatHide[Chat hidden/disabled]
```
