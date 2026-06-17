# PSAIDO Language Specification

> **SHELVED — not part of the active flow.** PSAIDO was an experiment in writing plans
> as a pseudo-code language for LLMs to translate. The drill flow now uses a plain
> Markdown brief the miner reads directly, dropping the psaido translation hop. This
> spec is kept for reference only; nothing in the active machine reads or writes psaido.

## Overview

PSAIDO is a language **for LLMs to write PSAIDO in**. It is not parsed or compiled — it is *read* by a model that translates it into real code. So it stays as small and flexible as possible: it describes a rough scaffold of *what* should happen and how the pieces connect, never *how* a given language implements it.

**Design principles**

- **Markdown is the language.** A module is just a Markdown file. Use Markdown's own tools — headings, prose, code fences, lists, links — instead of inventing syntax. The PSAIDO itself lives in a fenced block; everything around it is plain Markdown.
- **Prose carries intent.** Because a model reads this, natural-language description is first-class. When code would be awkward or over-specified, write a sentence. The reader fills in the obvious.
- **The reader is Agentic.** Capabilities like following an `@` link (search + read a file) are part of the language because the reader can do them. We don't build a resolver; we rely on the model.
- **Rough is fine.** This is a scaffold, not a contract. Leave detail to the translator's judgment. Add a keyword only when its absence causes real ambiguity.

---

## Module structure

Every `.pseudo` file is one small module: **YAML front matter** describing what it's for, then the code. The front matter gives a translator model the module's intent before it reads a single line.

```
---
module: auth/user
description: User identity — the User shape and lookups by id or email.
provides: [User, findById, findByEmail]
---

schema User
  id: number
  email: string

function findByEmail(input: string) -> User
  ...
```

- `module` *(required)* — the file's own path, matching how others `@`-link it.
- `description` *(required)* — one or two lines on what the module is for.
- `provides` *(optional)* — the names other modules link to via `#Name`.

Keep modules small and single-purpose. The `description` is the contract; the code is the detail.

---

## References (`@`)

`@` is the only import mechanism. It links one pseudo file to another; the reader resolves it by finding and reading the target — no build step, no resolver. The link *is* the instruction "read this for context," so never emit a literal `@` — translate each link into the target language's normal import or call.

```
@path/to/file          → the whole file (path/to/file.pseudo)
@path/to/file#Name     → just `Name` inside that file
```

Paths are relative to the project root; the `.pseudo` extension is implied. Use a link **inline**, where it is the import:

```
function login(input: Credentials) -> @auth/user#User
  record = @db/users#findByEmail(input.email)
  return record
```

…or alias a path once at the top and use the short name below:

```
import @db/users as udb

function lookup(id: number) -> udb.User
  return udb.findById(id)
```

---

## Schemas

Data shapes, built from the primitives `string`, `number`, `boolean`, `null`, `any`:

```
schema User
  id: number
  name: string
  email: string

schema Product
  id: number
  name: string
  price: number
```

Schemas can contain other schemas and arrays:

```
schema Order
  id: number
  customer: User
  items: [Product]
  total: number
```

---

## Functions

Declare input, output, and steps.

```
function getName(input: User) -> string
  return input.name

function addNumbers(input: {a: number, b: number}) -> number
  result = input.a + input.b
  return result
```

---

## Statements

Assignment, calls, construction, and indexing — all the everyday lines:

```
x = 5
name = "hello"
flag = true

user  = lookupUser(id)                  // call
first = items[0]                        // index
last  = items[items.length - 1]

user = User{ id: 1, name: "Alice" }     // construct
```

---

## Control flow

```
if x > 10 then
  y = 1
else
  y = 0

for item in items do
  total = total + item.price

while x < 100 do
  x = x + 1
```

---

## Example: a complete module

Everything together — front matter for intent, an `@` link for a dependency, schemas and functions for the scaffold:

```
---
module: orders/total
description: Compute and apply an order's total.
provides: [calculateOrderTotal, processOrder]
---

import @catalog/product as catalog

schema OrderItem
  product: catalog.Product
  quantity: number

schema Order
  id: number
  items: [OrderItem]
  total: number

function calculateOrderTotal(input: Order) -> number
  total = 0
  for item in input.items do
    total = total + item.product.price * item.quantity
  return total

function processOrder(input: Order) -> Order
  if input.items.length == 0 then
    return null
  input.total = calculateOrderTotal(input)
  return input
```

---

## Version

v1.0
