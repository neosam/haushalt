# API Structure

## API Endpoint Overview

```mermaid
flowchart TB
    subgraph "/api"
        subgraph "/auth"
            AR[POST /register]
            AL[POST /login]
            ALO[POST /logout]
            ARF[POST /refresh]
            AM[GET /me]
        end

        subgraph "/users"
            UG[GET /{id}]
            UP[PUT /{id}]
            USG[GET /me/settings]
            USP[PUT /me/settings]
        end

        subgraph "/households"
            HL[GET /]
            HC[POST /]
            HG[GET /{id}]
            HU[PUT /{id}]
            HD[DELETE /{id}]

            subgraph "/members"
                ML[GET /]
                MR[PUT /{user_id}/role]
                MP[POST /{user_id}/points]
                MD[DELETE /{user_id}]
            end

            subgraph "/settings"
                SG[GET /]
                SU[PUT /]
            end

            subgraph "/tasks"
                TL[GET /]
                TC[POST /]
                TG[GET /{task_id}]
                TU[PUT /{task_id}]
                TDD[DELETE /{task_id}]
                TS[GET /status]
                TD[GET /due]
                TA[GET /assigned]
                TP[GET /pending]
                TCO[POST /{task_id}/complete]
                TUC[DELETE /{task_id}/complete]
                TCA[POST /.../approve]
                TCR[POST /.../reject]
            end

            subgraph "/categories"
                CL[GET /]
                CC[POST /]
                CG[GET /{id}]
                CU[PUT /{id}]
                CDD[DELETE /{id}]
            end

            subgraph "/rewards"
                RL[GET /]
                RC[POST /]
                RG[GET /{id}]
                RU[PUT /{id}]
                RDD[DELETE /{id}]
                RP[POST /{id}/purchase]
                RAS[POST /{id}/assign]
                RUM[GET /users/me]
                RUA[GET /users]
                RPE[GET /pending]
            end

            subgraph "/punishments"
                PL[GET /]
                PC[POST /]
                PG[GET /{id}]
                PU[PUT /{id}]
                PDD[DELETE /{id}]
                PA[POST /{id}/assign]
                PCO[POST /{id}/complete]
                PUM[GET /users/me]
                PUA[GET /users]
                PPE[GET /pending]
                PO[GET /{id}/options]
                POA[POST /{id}/options/{option_id}]
                POD[DELETE /{id}/options/{option_id}]
                PPK[POST /user-punishments/{id}/pick]
            end

            subgraph "/invitations"
                IL[GET /]
                IC[POST /invite]
                IDD[DELETE /{id}]
            end

            subgraph "/chat"
                CHL[GET /]
                CHS[POST /]
                CHU[PUT /{id}]
                CHD[DELETE /{id}]
            end

            subgraph "/notes"
                NL[GET /]
                NC[POST /]
                NG[GET /{id}]
                NU[PUT /{id}]
                NDD[DELETE /{id}]
            end

            subgraph "/announcements"
                AL2[GET /]
                ALA[GET /active]
                AC[POST /]
                AG[GET /{id}]
                AU[PUT /{id}]
                ADD[DELETE /{id}]
            end

            subgraph "/activities"
                ACL[GET /]
            end

            subgraph "/leaderboard"
                LB[GET /]
            end

            subgraph "/point-conditions"
                PCL[GET /]
                PCC[POST /]
                PCG[GET /{id}]
                PCU[PUT /{id}]
                PCDD[DELETE /{id}]
            end
        end

        subgraph "/invitations"
            ILU[GET /]
            IA[POST /{id}/accept]
            IDC[POST /{id}/decline]
        end

        subgraph "/dashboard"
            DT[GET /tasks]
            DTD[GET /tasks/details]
            DTA[POST /tasks/{id}]
            DTR[DELETE /tasks/{id}]
            DTC[GET /tasks/{id}]
        end

        subgraph "/ws"
            WS[WebSocket /]
        end
    end
```

## HTTP Methods by Resource

```mermaid
flowchart LR
    subgraph Methods
        GET[GET<br/>Read]
        POST[POST<br/>Create/Action]
        PUT[PUT<br/>Update]
        DELETE[DELETE<br/>Remove]
    end

    subgraph Resources
        Users[Users]
        Households[Households]
        Tasks[Tasks]
        Rewards[Rewards]
        Punishments[Punishments]
    end

    GET --> Users & Households & Tasks & Rewards & Punishments
    POST --> Users & Households & Tasks & Rewards & Punishments
    PUT --> Users & Households & Tasks & Rewards & Punishments
    DELETE --> Households & Tasks & Rewards & Punishments
```

## Role-Based Access

```mermaid
flowchart TB
    subgraph Endpoints
        Public[Public Endpoints<br/>/auth/register<br/>/auth/login]
        Auth[Authenticated<br/>/auth/me<br/>/users/*<br/>/invitations/*<br/>/dashboard/*]
        Member[Member+<br/>/households/{id}/*<br/>read operations]
        Admin[Admin+<br/>/households/{id}/*<br/>manage operations]
        Owner[Owner Only<br/>/households/{id}/settings<br/>/members/*/role<br/>delete household]
    end

    subgraph Roles
        Anon[Anonymous]
        User[Authenticated User]
        M[Member]
        A[Admin]
        O[Owner]
    end

    Anon --> Public
    User --> Auth
    M --> Member
    A --> Admin
    O --> Owner

    O -.->|inherits| A
    A -.->|inherits| M
    M -.->|inherits| User
```

## Request/Response Flow

```mermaid
sequenceDiagram
    participant Client
    participant Router
    participant Middleware
    participant Handler
    participant Service
    participant DB

    Client->>Router: HTTP Request
    Router->>Middleware: Route matched
    Middleware->>Middleware: Auth check
    Middleware->>Handler: Inject AuthUser
    Handler->>Handler: Validate input
    Handler->>Service: Business logic
    Service->>DB: Query/Mutate
    DB-->>Service: Result
    Service-->>Handler: Domain object
    Handler-->>Client: JSON Response
```

## WebSocket Protocol

```mermaid
sequenceDiagram
    participant C as Client
    participant WS as WebSocket Server

    C->>WS: Connect /ws
    WS-->>C: Connected

    C->>WS: Authenticate {token}
    WS-->>C: Authenticated {user_id}

    C->>WS: JoinRoom {household_id}
    WS-->>C: JoinedRoom {household_id}

    C->>WS: SendMessage {content}
    WS-->>C: NewMessage {message}
    Note right of WS: Broadcast to room

    C->>WS: EditMessage {id, content}
    WS-->>C: MessageEdited {message}

    C->>WS: DeleteMessage {id}
    WS-->>C: MessageDeleted {id}

    C->>WS: LeaveRoom {household_id}
    WS-->>C: LeftRoom

    C->>WS: Ping
    WS-->>C: Pong
```

## Error Response Format

```mermaid
flowchart LR
    subgraph "Error Response"
        Code[HTTP Status Code]
        Body[JSON Body]
    end

    subgraph "Status Codes"
        C400[400 Bad Request]
        C401[401 Unauthorized]
        C403[403 Forbidden]
        C404[404 Not Found]
        C429[429 Too Many Requests]
        C500[500 Internal Error]
    end

    subgraph "Body Structure"
        Err["{ error: string }"]
    end

    Code --> C400 & C401 & C403 & C404 & C429 & C500
    Body --> Err
```

## Pagination Pattern

```mermaid
flowchart LR
    subgraph Request
        Limit[limit: number]
        Before[before: cursor]
        After[after: cursor]
    end

    subgraph Response
        Data[data: array]
        HasMore[has_more: bool]
        NextCursor[next_cursor: string?]
    end

    Request --> API[API Endpoint]
    API --> Response
```
