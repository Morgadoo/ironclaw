# Boot Checklist — Project E

## Startup Sequence

1. [ ] Load layered configuration (default.toml → instance.toml → adaptive.toml)
2. [ ] Verify PostgreSQL connection and run pending migrations
3. [ ] Connect to NATS JetStream event bus
4. [ ] Initialize LLM Health Monitor (circuit breaker)
5. [ ] Load module registry from PostgreSQL
6. [ ] Build BM25 index from active modules
7. [ ] Initialize perception collectors (Firecrawl, Tavily)
8. [ ] Start Python scraper NATS consumers
9. [ ] Initialize forge pipeline (Wasmtime sandbox)
10. [ ] Load skill catalog from filesystem
11. [ ] Start Supervisor agent
12. [ ] Compute initial health indicators
13. [ ] Emit CycleStarted event
14. [ ] Begin first cognitive cycle

## Health Verification

After boot, verify:
- All five health indicators are computable (may be zero on first run)
- Event bus publish/subscribe works (echo test)
- LLM provider responds to probe
- Module registry is accessible
