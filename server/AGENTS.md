# AGENTS.md (server)

Instructions for agents working on the Symfony server. **Read the repository's root `AGENTS.md` first**: language, documentation, operations, glossary and secrets rules apply here too. This file only adds what is specific to PHP and Symfony.

The conventions come from the maintainer's other Symfony projects (Riddle, and rpgutilities before it), adapted to this repository. Where this project deliberately differs, it says so.

## What the server is

The optional authoritative server (docs/DECISIONS.md D38–D40): accounts, several libraries per server, roles, and applying the operations clients push. The plan is Phase 7 ("La nube") in `docs/ROADMAP.md`; the earlier server plan in `docs/historico/` is historical. Setup, ports and databases: `docs/development/server.md`.

## Working rules

- **Plan before code.** A phase starts from a plan document in `docs/plans/`. Open questions go there, and once answered, into `docs/DECISIONS.md`. When the phase is done, the plan moves to `docs/historico/` and is no longer edited.
- **Verify against the code, not against the docs.** Before claiming something works or exists, check it (`grep`, `make console C="debug:router"`, a query). Docs get rewritten and drift.
- **When revising a closed decision, say so out loud** and mark it in `DECISIONS.md`. A silent change is how nobody knows, three months later, what was decided and why.
- **The `Makefile` is the source of truth for commands** (`make help`). Everything runs in Docker; never run PHP or Composer on the host, and never hand-write `docker exec … sh -c 'cd …'`.
- **`make check` must pass** before a commit.
- **Commits:** unlike Riddle and rpgutilities, in this repository the agent may make *local* commits, one per milestone, when the task includes it. **Never push**, open PRs, or write anything remote.

## Where each class goes

`src/` is a flat `App\` namespace, no bundles. **No class goes in the root of `src/`** except `Kernel.php`: every class lives in a folder that names its role.

| Folder | What goes there |
|---|---|
| `Controller/` | Controllers, thin: they map the request, call a service and return JSON. The API is versioned by route (`/v1/…`) |
| `Entity/` | **Only Doctrine entities.** An enum is not an entity |
| `Repository/` | One Doctrine repository per entity |
| `Enum/` | Enums, even when a single class uses them |
| `Dto/` | Transport objects and value objects: request payloads, responses, values passed between layers |
| `Validator/` | Custom constraints and their validators (`ValidOperation`) |
| `Security/` | Account services (`EmailCodeIssuer`, `AccountMailer`, `RequestLimiter`, `ActivityLog`, the only writer of the append-only activity log, D66), authenticators and voters (Phase 7) |
| `EventListener/` | Kernel event listeners, e.g. `AccountExceptionListener` turning a business exception into its JSON response |
| `Command/` | Console commands |
| `Scheduler/` | Scheduled tasks (`symfony/scheduler`): an `#[AsSchedule]` provider, and the message + `#[AsMessageHandler]` it dispatches, when the job is small enough not to need its own `Message/`/`MessageHandler/` split. Today: `PurgeSchedule` runs `App\Security\AccountPurger` hourly (D49) |
| `Sync/` | Everything about applying operations from clients. Today: `OperationSignature` (checks the author's Ed25519 signature, port of `crates/domain/src/signature.rs`) and its exception. `SyncNotifier`, `EventsToken`, `NoticeTopic` and `HubSettings` (live notices over Mercure, milestone 7.3). `Sync/Merge/` (milestone 7.4.4) is the engine that applies operations to the catalog tables, the port of `sync::merge`: `CatalogMerge` (the interface), `DbalCatalogMerge` (the rules) and `CatalogSql` (the only class with SQL, built from the entities' metadata). Nothing calls it yet but `app:catalog:rebuild`. `Sync/Invitation/` (milestone 7.6.1, D43) holds the invitations to a cloud library: `InvitationRules` (who may invite and which person, asked again on accepting), `InvitationManager` (create, list, revoke), `InvitationLookup` (what a link shows without a session), `InvitationAcceptor` (the transaction under the library lock), and the small `InvitationToken`, `EmailFingerprint` and `InvitationSettings`; `OperationActivity` reads what an applied operation says for the activity log (7.7) |
| `Metadata/` | Looking books up in an outside catalogue for accounts (7.8, D83): `BookProvider` (the interface), `GoogleBooksHttpProvider`, `BookLookup` (cache, limits and the pause after a quota refusal), `Isbn` and `BookMetadata` (the fixed shape). An engine folder, like `Sync/` |
| `DataFixtures/` | Dev and test only (`make fixtures`, docs/development/fixtures.md): the scenario catalogue of the cloud, loaded through the real services, and `SigningDevice`, which builds and signs operations for the fixtures and the tests alike |

`Enum/`, `Dto/`, `Repository/` and `Validator/` are standard Symfony convention and are used without asking. If something fits no folder and is not a plain utility, **ask before inventing a category**. An engine folder like `Sync/` is legitimate (Riddle has `Generator/`), but it gets documented here and in `docs/architecture/` when it is created.

`tests/` mirrors `src/`. Test helpers go in `tests/Support/`.

## Where the operation rules live

Operations are the contract with every client, so their rules are defined in `spec/` (mounted read-only at `/var/www/spec`) and implemented twice: in Rust (`crates/domain`) and here. **A rule change goes to `spec/` first**, with conformance cases, and then to both implementations.

| Piece | File |
|---|---|
| Shape of an operation | `Dto/OperationPayload.php`, `Dto/ChangePayload.php` |
| Entities and their fields (mirror of `schema.rs`) | `Enum/CatalogEntity.php` |
| Spec rules and error codes | `Validator/ValidOperation.php` and its validator; codes in `Enum/OperationErrorCode.php` |
| Signatures (D56) | `Sync/OperationSignature.php`; codes in `Enum/SignatureErrorCode.php` |
| Conformance cases | `tests/Validator/ValidOperationValidatorTest.php` and `tests/Sync/OperationSignatureTest.php` |

It is split in two layers, as in the Rust client. The **Serializer** rejects anything without the shape of an operation (missing or unknown keys, wrong JSON types, unknown entity or action). The **`ValidOperation` constraint** applies the spec rules.

`ValidOperation` checks by hand, unlike the rest of the input validation. This is deliberate: the spec fixes the error codes and the order in which rules are checked, only the first error is reported, and the result must match the Rust client case by case. Per-property `#[Assert]` attributes would report every violation, in their own order and with Symfony's codes. The rest of the input validation follows the next section.

`CatalogEntity`, not `Entity`: the rows that operations touch are not Doctrine entities, and the name must not suggest they are.

## Input: DTOs and the framework, never by hand

**If Symfony already has the mechanism, use Symfony's.** Riddle had to redo a controller with `json_decode` and seven hand-written `if`s.

- Request bodies: a `readonly` DTO in `Dto/` with `#[Assert\…]` attributes, bound with `#[MapRequestPayload]` (or `#[MapQueryString]`). Invalid payloads never reach the controller: the framework answers **400** for broken JSON and **422** for validation errors.
- `#[MapRequestPayload(type: X::class)]` on an `array` argument maps a **list** of DTOs.
- Strict payloads: pass `serializationContext: OperationPayload::SERIALIZATION_CONTEXT` (or an equivalent constant) so unknown keys are rejected, as the Rust client does.
- The wire format is `snake_case`. When a property name differs, declare it with `#[SerializedName('…')]` property by property, not with a global name converter.
- The API is JSON, so no `symfony/form` there. Registration, email confirmation and password reset happen in the app, with a code sent by email (D49, D50). The only planned HTML is the admin panel (EasyAdmin, D52).

## Errors

- A method **returns its type or throws**. Never `Foo|string` or a nullable to signal failure: the caller should not have to inspect the type to know whether something went wrong.
- **Custom exceptions**, not bare `\RuntimeException` or `\LogicException`: whoever catches needs to tell failures apart.
- Validation failures are constraint violations, with a stable `code` the client can act on (the spec codes for operations).

## PHP conventions

These come from the official `AGENTS.md` of `symfony/ai` and EasyAdminBundle.

- `declare(strict_types=1)` in every file. This is an application, not a bundle, so the EasyAdmin objection does not apply. Rector and php-cs-fixer both enforce it.
- `final` classes by default; `readonly` for DTOs and value objects. A service that may become `lazy: true` is not `readonly`, because a lazy proxy cannot extend a readonly class.
- Constructor property promotion.
- **No `empty()`.** Compare explicitly: `[] === $array`, `'' === $string`, `null === $value`.
- Yoda comparisons (`null === $value`); php-cs-fixer enforces them.
- **Early returns** instead of `else` after a `return` or `throw`.
- **Array shapes in phpdoc** for parameters and returns (`@return list<array{id: string, role: string}>`). PHPStan needs them, and they document the code.
- Enums rather than loose constants for fixed sets of values. `SCREAMING_SNAKE_CASE` for constants, `camelCase` for methods and variables.
- A newline at the end of every file.

## Symfony conventions

- Attributes for framework metadata: `#[Route]`, `#[MapRequestPayload]`, `#[IsGranted]`, `#[Assert\…]`, `#[AsCommand]`, `#[AsEventListener]`, `#[Autowire]`, `#[Target]`. No YAML or XML routing. A YAML service definition is the last resort.
- Autowiring and autoconfiguration; controllers extend `AbstractController`.
- New capabilities come from `composer require` and the Flex recipe. Do not hand-edit `config/bundles.php` or hand-write a bundle's base config.
- Before hand-writing infrastructure (locks, rate limiting, HTTP clients, password hashing, tokens), check whether a Symfony component covers it. It usually does.
- **Persistence:** Doctrine ORM for everything, the catalog included (D92): one entity per catalog table in `src/Entity/Catalog/`, each with a key of the server's own; the app's id is only unique inside its library. The one exception is the merge engine (`Sync/Merge/`, D92 as revised on 2026-10-09): it writes with DBAL, and its SQL is built from the entities' Doctrine metadata by a single class, never written per table. Anything else reads and writes the catalog through the ORM. Plain SQL only where the ORM cannot do the job, kept inside one class and said in a comment (`CloudOperationRepository`, whose JSON must stay byte-faithful). Do not copy the Rust client's shape: same rules, PHP's own patterns.
- **Schema changes always go through a migration** (`make migration`, then `make migrate`), never `doctrine:schema:update`. Remove the generated comments and fill in `getDescription()`. Name indexes explicitly.

## Environment files

- `.env` is committed and **holds no credentials**, only defaults and placeholders. Development credentials go in `.env.local`, which is not versioned (see `docs/development/server.md`). Production secrets go in `.env.local` on the server.
- `.env.test` **is** committed, and has to be: Symfony does not load `.env.local` when `APP_ENV=test`, so without it a fresh clone cannot run the suite. Its `test` role exists only inside the container and can only reach `kosmotheka_test`.
- **No `APP_SECRET` is committed**, not even for development: it goes in `.env.local`. Tests make up a random one in `tests/bootstrap.php`.
- There is **no `compose.yaml` in `server/`**: `extra.symfony.docker` is `false` in `composer.json`, so recipes do not generate one. The Docker setup lives in `server/docker/`. If a `compose.yaml` ever appears, delete it.

## Tests

- **A new endpoint gets a functional test** (`WebTestCase`), and the rejection paths matter as much as the happy one. Services get `KernelTestCase` tests that go through the container, the way a caller would.
- `#[DataProvider('name')]` as an attribute. The `@dataProvider` annotation is deprecated, and `phpunit.dist.xml` fails on deprecations.
- The test database is `kosmotheka_test` (Doctrine adds the `_test` suffix); `make db-test` migrates it.
- Operation rules are tested through the Serializer and the Validator, the same path a request takes, against the shared cases in `spec/conformance/`.

## Quality

`make check` runs four tools whose jobs do not overlap; `make fix` applies style and Rector, then verifies.

| Tool | What it covers |
|---|---|
| php-cs-fixer | Style: `@Symfony` + `@Symfony:risky`, the ruleset of the official Symfony repositories |
| Rector | Syntax modernisation: PHP 8.5 sets, promotion, types, Symfony and Doctrine quality sets |
| PHPStan | Correctness: the **highest level** (`max`), with phpstan-doctrine (also validates DQL), phpstan-symfony (reads the test container, so `make phpstan` warms it first) and phpstan-phpunit. **No baseline**, deliberately. Where a value can legitimately be missing, narrow it (`?? self::fail()`, an assertion), never silence the rule |
| PHPUnit | Behaviour |

- A style warning is not fixed by hand: `make fix` fixes it.
- A file generated by a recipe (`tests/bootstrap.php`) is excluded from analysis, not patched, because a patch is lost on the next recipe update.

## Known traps

- **The Serializer ignores `@param list<SomeDto>` unless both `phpstan/phpdoc-parser` and `phpdocumentor/type-resolver` are installed.** With only one of them, nested DTOs arrive as plain arrays and fail later with a `TypeError`. Both are installed; do not remove either.
- **Clear the cache after `composer require`** when a new package changes how services are wired (`make cc`, or delete `var/cache/test` for the test environment).
- **The LSP index goes stale after `composer require`.** Before "fixing" a suspicious "undefined type/method", check it against `vendor/`.
- **`dama/doctrine-test-bundle`'s recipe is a contrib recipe**, skipped by Flex because `allow-contrib` is `false`. It is registered by hand in `config/bundles.php` (test env only) and in `phpunit.dist.xml` (`DAMA\DoctrineTestBundle\PHPUnit\PHPUnitExtension`), the one deliberate exception to "let Flex do it".

## Discover, don't guess

Framework APIs change between versions, and training data may be stale. Look things up in the project:

- `make console C="about"`, `debug:router`, `debug:container`, `debug:autowiring <name>`, `debug:config <bundle>`, `config:dump-reference <bundle>`.
- `make console C="lint:container"` and `lint:yaml config/`.
- The installed source and docblocks under `vendor/`, and https://symfony.com/doc/current/ for the version in `composer.json` (8.1). 8.1 is not an LTS release: the next LTS is 8.4 (November 2027), and until then minor upgrades come every six months.
