# Frontend Architecture

## Component Structure

```mermaid
flowchart TB
    subgraph App["Leptos App (WASM)"]
        Router[Router]

        subgraph Pages
            Login[LoginPage]
            Register[RegisterPage]
            Dashboard[DashboardPage]
            Household[HouseholdPage]
            HHSettings[HouseholdSettingsPage]
            Tasks[TasksPage]
            Rewards[RewardsPage]
            Punishments[PunishmentsPage]
            Chat[ChatPage]
            Notes[NotesPage]
            Activity[ActivityPage]
            Settings[SettingsPage]
            UserSettings[UserSettingsPage]
        end

        subgraph Components
            Header[Header]
            Nav[Navigation]
            TaskList[TaskList]
            TaskForm[TaskForm]
            TaskDetailModal[TaskDetailModal]
            RewardCard[RewardCard]
            MemberList[MemberList]
            Leaderboard[Leaderboard]
            ChatWindow[ChatWindow]
            NoteEditor[NoteEditor]
        end

        subgraph Services
            API[ApiClient]
            Auth[AuthService]
            WS[WebSocketClient]
            i18n[i18n Service]
        end
    end

    Router --> Pages
    Pages --> Components
    Pages --> Services
    Components --> Services
```

## Page Routing

```mermaid
flowchart LR
    subgraph "Public Routes"
        L[/login]
        R[/register]
    end

    subgraph "Protected Routes"
        D[/dashboard]
        H[/households/:id]
        HS[/households/:id/settings]
        T[/households/:id/tasks]
        RW[/households/:id/rewards]
        P[/households/:id/punishments]
        C[/households/:id/chat]
        N[/households/:id/notes]
        A[/households/:id/activity]
        S[/settings]
        US[/settings/user]
    end

    Guard{Auth Guard}
    L & R --> Guard
    Guard -->|authenticated| D
    Guard -->|not authenticated| L
    D --> H --> HS & T & RW & P & C & N & A
    D --> S --> US
```

## State Management

```mermaid
flowchart TB
    subgraph "Global State"
        AuthState[AuthState<br/>user, tokens]
        LangState[LanguageState<br/>current locale]
    end

    subgraph "Page State"
        HHState[HouseholdState<br/>current household]
        TaskState[TaskState<br/>tasks, filters]
        ChatState[ChatState<br/>messages, ws connection]
    end

    subgraph "Reactive Signals"
        Signal1[create_signal]
        Signal2[create_resource]
        Signal3[create_effect]
    end

    AuthState --> Signal1
    LangState --> Signal1
    HHState --> Signal2
    TaskState --> Signal2
    ChatState --> Signal1 & Signal3
```

## API Client Architecture

```mermaid
flowchart TB
    subgraph ApiClient
        Base[Base HTTP Client]

        subgraph Modules
            AuthApi[auth()]
            UserApi[users()]
            HHApi[households()]
            TaskApi[tasks()]
            RewardApi[rewards()]
            PunishApi[punishments()]
            ChatApi[chat()]
            NoteApi[notes()]
            AnnApi[announcements()]
            InvApi[invitations()]
            ActApi[activities()]
            DashApi[dashboard()]
        end
    end

    subgraph "Request Flow"
        Req[Request] --> Token[Add Auth Token]
        Token --> Send[Send Request]
        Send --> Resp{Response}
        Resp -->|401| Refresh[Refresh Token]
        Refresh --> Retry[Retry Request]
        Resp -->|Success| Parse[Parse JSON]
    end

    Base --> Modules
    Modules --> Req
```

## i18n System

```mermaid
flowchart LR
    subgraph Translations
        EN[en.json]
        DE[de.json]
    end

    subgraph i18n["i18n Service"]
        Load[Load Translations]
        Get[get_translation]
        Format[format with params]
    end

    subgraph Usage
        Comp[Component]
        T["t!(key)"]
    end

    EN & DE --> Load
    Load --> Get
    Get --> Format
    Comp --> T --> Get
```

## Component Communication

```mermaid
flowchart TB
    Parent[Parent Component]
    Child1[Child Component 1]
    Child2[Child Component 2]

    subgraph Props
        P1[Props Down]
    end

    subgraph Callbacks
        C1[Callbacks Up]
    end

    subgraph Context
        Ctx[Provide/Use Context]
    end

    Parent -->|props| Child1 & Child2
    Child1 & Child2 -->|callbacks| Parent
    Parent -.->|provide| Ctx
    Child1 & Child2 -.->|use| Ctx
```
