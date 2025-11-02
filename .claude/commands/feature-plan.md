---
name: feature-plan
description: Generate implementation plan from feature specification
---

I will help you create a detailed implementation plan from a feature specification, including technical approach, architecture decisions, data models, and dependencies.

**Usage**:
- `/feature-plan <feature-id>` - Create implementation plan from spec
- `/feature-plan --show <feature-id>` - Display existing plan
- `/feature-plan --update <feature-id>` - Update existing plan

**Instructions for me**:

1. **Load feature spec**:
   - Read `.mnemosyne/artifacts/specs/<feature-id>.md`
   - If not found: "Error: Feature spec '<feature-id>' not found. Use /feature-specify first."
   - Parse YAML frontmatter to get spec memory_id
   - Extract user scenarios, requirements, and constraints

2. **Check for existing plan**:
   - Look for `.mnemosyne/artifacts/plans/<feature-id>-plan.md`
   - If exists:
     - Parse version from frontmatter
     - If `--show`: Display plan summary and exit
     - If `--update`: Increment version and update
     - Otherwise: Ask if user wants to view, update, or create new version

3. **Load constitution** (if exists):
   - Read `.mnemosyne/artifacts/constitution/project-constitution.md`
   - Extract architecture decisions, constraints, quality gates
   - Use to guide implementation choices

4. **Gather implementation details**:
   Ask user about:

   a) **Technical Approach**:
   - High-level strategy (e.g., "REST API with JWT auth", "Event-driven with message queue")
   - Why this approach? (rationale)
   - Alternative approaches considered and rejected

   b) **Architecture Decisions**:
   For each significant choice:
   - Decision: [What was decided]
   - Rationale: [Why this choice]
   - Trade-offs: [What we gain vs. what we sacrifice]
   - Example: "Use Redis for caching - improves latency (gain) but adds operational complexity (trade-off)"

   c) **Data Models** (if applicable):
   - Database schema (tables, columns, relationships)
   - Data structures (classes, structs, interfaces)
   - Data flow diagrams

   d) **API Design** (if applicable):
   - Endpoints and methods
   - Request/response formats
   - Error handling strategy

   e) **Dependencies**:
   - External libraries/frameworks needed
   - Internal modules that must be modified
   - Services or APIs consumed

   f) **Integration Points**:
   - What existing systems need to be integrated?
   - How will they communicate? (REST, GraphQL, events, RPC)
   - Authentication/authorization requirements

   g) **Testing Strategy**:
   - Unit testing approach
   - Integration testing plan
   - E2E testing scenarios
   - Performance testing requirements

   h) **Risks and Mitigations**:
   - What could go wrong?
   - How will we handle it?

5. **Format plan as markdown**:
   ```markdown
   ---
   type: implementation_plan
   id: <feature-id>-plan
   name: <feature-name> Implementation Plan
   feature_id: <feature-id>
   version: 1.0.0
   created_at: <ISO 8601 timestamp>
   updated_at: <ISO 8601 timestamp>
   memory_id: <will be filled>
   references: [<spec-memory-id>, <constitution-memory-id>]
   ---

   # Implementation Plan: <feature-name>

   ## Technical Approach

   **Strategy**: [High-level approach]

   **Rationale**: [Why this approach]

   **Alternatives Considered**:
   - [Alternative 1]: Rejected because [reason]
   - [Alternative 2]: Rejected because [reason]

   ## Architecture Decisions

   ### [Decision Area 1]

   **Decision**: [What was decided]

   **Rationale**: [Why this choice]

   **Trade-offs**:
   - **Gain**: [Benefit]
   - **Sacrifice**: [Cost/complexity]

   ### [Decision Area 2]

   ...

   ## Data Models

   ### Database Schema

   ```sql
   CREATE TABLE users (
     id UUID PRIMARY KEY,
     email VARCHAR(255) UNIQUE NOT NULL,
     created_at TIMESTAMP DEFAULT NOW()
   );
   ```

   ### Data Structures

   ```rust
   struct User {
     id: Uuid,
     email: String,
     created_at: DateTime<Utc>,
   }
   ```

   ## API Design

   ### Endpoints

   #### POST /api/auth/login

   **Description**: Authenticate user and issue JWT

   **Request**:
   ```json
   {
     "email": "user@example.com",
     "password": "secure_password"
   }
   ```

   **Response** (200 OK):
   ```json
   {
     "access_token": "eyJ...",
     "expires_in": 86400
   }
   ```

   **Errors**:
   - 401: Invalid credentials
   - 429: Rate limit exceeded

   ## Dependencies

   ### External Libraries
   - `jsonwebtoken` (0.9.x) - JWT signing/verification
   - `bcrypt` (0.15.x) - Password hashing
   - `redis` (0.24.x) - Session storage

   ### Internal Modules
   - `user-service`: Modified to support JWT validation
   - `auth-middleware`: New module for request authentication

   ### External Services
   - Redis: Session store and rate limiting
   - PostgreSQL: User database

   ## Integration Points

   ### User Service Integration

   **Communication**: REST API

   **Authentication**: Internal API key

   **Endpoints Used**:
   - `GET /internal/users/:id` - Fetch user details
   - `POST /internal/users/:id/sessions` - Create session

   ## Testing Strategy

   ### Unit Tests
   - JWT generation and validation
   - Password hashing and verification
   - Token expiration logic

   **Target Coverage**: 90%+ for auth logic

   ### Integration Tests
   - Full login flow (email/password → JWT)
   - Token refresh flow
   - Rate limiting enforcement

   ### E2E Tests
   - User login → protected resource access
   - Token expiration → re-authentication
   - Invalid credentials → error handling

   ### Performance Tests
   - Login endpoint: <100ms p95 latency
   - Token validation: <10ms p95 latency
   - Load test: 1000 concurrent logins

   ## Risks and Mitigations

   ### Risk: JWT secret compromise

   **Impact**: All tokens can be forged

   **Mitigation**:
   - Use RS256 (asymmetric keys) instead of HS256
   - Rotate keys regularly (every 90 days)
   - Store private key in secure vault

   ### Risk: Redis downtime

   **Impact**: Sessions unavailable, users logged out

   **Mitigation**:
   - Redis cluster with replication
   - Graceful degradation: Allow stateless JWT validation
   - Monitor Redis health and alert on failures

   ## Implementation Order

   1. **Foundation** (Week 1):
      - Database schema creation
      - User model and repository
      - Password hashing utilities

   2. **Core Auth** (Week 1-2):
      - JWT generation and validation
      - Login endpoint implementation
      - Token refresh mechanism

   3. **Integration** (Week 2):
      - Middleware for protected routes
      - User service integration
      - Redis session storage

   4. **Polish** (Week 3):
      - Rate limiting
      - Error handling improvements
      - Comprehensive testing

   ## Success Criteria

   - [ ] All P0 user scenarios passing
   - [ ] Performance targets met (p95 latency)
   - [ ] 90%+ test coverage on auth logic
   - [ ] Security review completed (no critical findings)
   - [ ] Load testing passed (1000 concurrent users)

   ## Open Questions

   - [ ] Password complexity requirements?
   - [ ] Session duration for "remember me"?
   - [ ] Multi-factor authentication in scope?

   ## Related

   - Feature Spec: `.mnemosyne/artifacts/specs/<feature-id>.md`
   - Constitution: `.mnemosyne/artifacts/constitution/project-constitution.md`
   - Clarifications: `.mnemosyne/artifacts/clarifications/<feature-id>-clarifications.md`
   ```

6. **Write plan file**:
   - Create `.mnemosyne/artifacts/plans/<feature-id>-plan.md`
   - Ensure directory exists
   - If updating: Increment version appropriately

7. **Store memory entry**:
   - Use Mnemosyne CLI: `mnemosyne remember`
   - Arguments:
     - Content: "Implementation plan for <feature-name>: <technical approach summary> ...see .mnemosyne/artifacts/plans/<feature-id>-plan.md for full plan"
     - Namespace: `project:<project-name>`
     - Importance: 8 (plans are important)
     - Type: implementation_plan
     - Tags: plan,<feature-id>,architecture
     - Context: "Implementation plan for <feature-name>"
   - Capture memory_id

8. **Create memory links**:
   - Link plan → spec with relationship "implements"
   - Link plan → constitution with relationship "guided_by"
   - Update plan's `references` field with memory IDs
   - Update plan's `memory_id` field

9. **Display confirmation**:
   ```
   ✓ Implementation plan created successfully

   Feature ID: <feature-id>
   Name: <feature-name>
   Location: .mnemosyne/artifacts/plans/<feature-id>-plan.md
   Memory ID: <memory-id>

   Architecture Decisions: <count>
   Dependencies: <count>
   Integration Points: <count>
   Risks Identified: <count>

   Implementation Timeline: <duration>

   Alignment:
   - Feature Spec: ✓ Covers all P0/P1 scenarios
   - Constitution: ✓ Follows architecture decisions
   - Clarifications: ✓ Incorporates all resolved decisions

   Next steps:
   - Review plan: cat .mnemosyne/artifacts/plans/<feature-id>-plan.md
   - Break down tasks: /feature-tasks <feature-id>
   - Create git branch: git checkout -b feature/<feature-id>
   - Start implementation
   ```

10. **Error handling**:
    - If spec not found: "Error: Feature spec '<feature-id>' not found"
    - If plan exists and no flag: Offer to view/update/create new
    - If technical details too vague: Ask for more specifics
    - If missing critical decisions: Prompt for architecture choices

**Special behaviors**:
- `--show`: Display existing plan summary (architecture decisions, dependencies, timeline)
- `--update`: Increment version, ask what changed, preserve unchanged sections
- Smart defaults: Pre-fill common patterns based on tech stack in constitution
- Validation: Require at least 1 architecture decision, 1 dependency, testing strategy
- Constitution alignment: Flag if plan violates constitution principles
- Completeness check: Ensure all spec requirements addressed in plan

**Examples**:
```
/feature-plan jwt-authentication
/feature-plan --show jwt-authentication
/feature-plan --update jwt-authentication
```

Please proceed to create the implementation plan based on user input and feature spec.
