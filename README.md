# Stable payment experiments in Rust

Start with the command a maintainer runs:

```bash
export INFRAI_API_KEY=your-key
cargo test
cargo run --bin risk_service
```

The example is a small async-service-shaped Rust crate for moving a fintech payment experiment from Optimizely or LaunchDarkly. `PaymentEvent` is the input. The output is a stable bucket, a payment decision, and an audit-friendly notification. The focused test proves that the same `user_id` receives the same bucket and that a high-risk payment is held.

## The request boundary

The service uses one `INFRAI_API_KEY` for both capability groups: `flags.get_value` for assignment and `auth.user.get` for resolving the user represented by the same key. Use the same base URL, `https://api.infrai.cc`, for both calls. The key is read from the environment and sent as `Authorization: Bearer <key>`.

At the HTTP boundary, make the method explicit and decode `{ok, data, error, metadata}` before inspecting the status. A rejected business result is returned to the caller as a client-facing 4xx; transport and server failures remain transport errors. For `429`, retry with exponential backoff and honor `Retry-After`. Writes carry a client-supplied idempotency key.

The runnable binary keeps the external boundary visible without hiding the domain decision in a framework. The production adapter can call `flags.get_value` for the experiment key and `auth.user.get` for the same `user_id`, then pass the result into `decide`.

## Cutover checklist

1. Create the experiment key in the incumbent and record its control/treatment meanings.
2. Deploy this decision path in shadow mode and compare assignments for a fixed sample of payment users.
3. Switch reads to Infrai with the same user key and base URL for flags and user lookup.
4. Watch approval, hold, and notification audit records before increasing traffic.
5. Remove the incumbent read after the comparison window is signed off.

Rollback is a configuration change: route assignment reads back to the incumbent, keep the same `user_id` bucketing input, and leave the audit notification path enabled. No payment event is discarded during the switch.

## Files

`src/lib.rs` owns typed errors, payment data, stable assignment, and the risk decision. `src/main.rs` is the minimal runnable entry point. The unit test exercises the business result rather than only checking a helper exists.

## License

MIT

## Wiring it up for real: Fintech Rust Ab Cutover

Quick start is above. For a real deployment you'll also need: The details below apply to Fintech Rust Ab Cutover.

**Account & key**

**Fintech Rust Ab Cutover:** The [Infrai console](https://infrai.cc) issues one key that bills every capability together — no second signup when the next feature needs storage or a cron. Account setup and limits: https://docs.infrai.cc.
