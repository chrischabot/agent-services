---
description: Authorize a channel subject for project access or sending.
---

Authorize a channel subject through Arcwell MCP.

Use `channel_authorize` with:

- `channel`: channel name, such as `telegram`.
- `subject`: exact subject, such as `telegram:chat:123`, `telegram:user:456`, or `telegram:@username`.
- `can_read_projects`, `can_write_projects`, and `can_send` booleans.

Default to the narrowest permission needed. Do not grant project write access to a whole chat unless the user explicitly asks for that chat to control project context.
