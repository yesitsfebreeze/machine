# Attribution

`board` is an original, zero-dependency implementation. **No code** from any other
project was copied into it.

Its data model (projects -> columns -> cards -> comments), its realtime event shape,
and its left-to-right kanban UX were used as a **design template** from:

- **LocalBoards** — github.com/florian-strasser/LocalBoards (MIT License).

LocalBoards is a Nuxt 4 SSR app hard-wired to MySQL, with ~20 npm dependencies and a
build step. `board` borrows only the *shape* of its domain (flattened from its MySQL
tables, dropping users/invitations/notifications/sessions/api-keys/attachments) and
the *names* of its realtime events — it shares none of its code, runtime, or
dependencies. The design credit above satisfies the courtesy owed to an MIT-licensed
template even though no MIT-covered code is redistributed.
