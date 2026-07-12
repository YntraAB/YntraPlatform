---
name: db-test-verification
description: Guidelines for writing thread-safe database tests, verifying transactional isolation, and configuring SQLite WAL mode in test suites.
---

# Database Test & Verification Guide

This skill defines the requirements, patterns, and safety constraints for writing automated integration and regression tests against the SQLite/libSQL database layer.

---

## 1. WAL Mode for Concurrent Test Runtimes

* **Requirement**: Use the thread-safe global connection model or verify that custom test connections explicitly activate WAL mode.
* **Why**: Without WAL (Write-Ahead Logging), concurrent tests will block each other's writes, resulting in database lock failures.
* **Instruction**: Execute `PRAGMA journal_mode = WAL;` immediately after creating test connections.

---

## 2. Test Transaction Isolation

* **Isolation Rule**: Automated tests must never contaminate the local database file or leak state changes to other tests.
* **Implementation**:
  1. Wrap individual test sequences in a transaction (`BEGIN TRANSACTION;`).
  2. Run test mutations and assertions.
  3. Rollback changes (`ROLLBACK;`) upon test termination.
* **Refer to Example**: See [examples/db_test_isolation.rs](file:///c:/Users/hellich/Desktop/YntraPlatform/.agents/skills/db-test-verification/examples/db_test_isolation.rs) for a complete Rust testing isolation setup.

---

## 3. Mocking Observers

* Since database mutations notify UI listeners through the `DatabaseObserver` callback interface, mock the observer interface inside tests to verify that correct change-event triggers are fired when specific tables are mutated.
