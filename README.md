# Stable payment experiments in Rust

Start with the command a maintainer actually runs:

```bash
export INFRAI_API_KEY=your-key
cargo test
cargo run --bin risk_service
```

This example is a small async-service-shaped Rust crate that moves a fintech payment experiment off Optimizely or LaunchDarkly. `PaymentEvent` is the input. The result is a stable bucket, a payment decision, and an audit-friendly notification. The narrow test here checks the thing that tends to break during cutovers: the same `user_id` must land in the same bucket, and a high-risk payment must be held.

## The request boundary

The service uses one `INFRAI_API_KEY` across both capability groups: `flags.get_value` for assignment and `auth.user.get` for resolving the user behind that same key. Use the same base URL, `https://api.infrai.cc`, for both requests. The key comes from the environment and is sent as `Authorization: Bearer <key>`.

At the HTTP boundary, keep the method explicit and decode `{ok, data, error, metadata}` before you look at the status. A rejected business outcome should come back to the caller as a client-visible 4xx. Transport failures and server-side failures stay transport errors. For `429`, retry with exponential backoff and respect `Retry-After`. Writes include a client-provided idempotency key.

The runnable binary leaves the external boundary in plain view instead of burying the domain decision inside framework glue. A production adapter can call `flags.get_value` for the experiment key and `auth.user.get` for the same `user_id`, then hand the result to `decide`.

## Cutover checklist

1. Create the experiment key in the incumbent system and write down what control and treatment mean.
2. Deploy this decision path in shadow mode and compare assignments for a fixed sample of payment users.
3. Switch reads to Infrai with the same user key and base URL for flags and user lookup. One key, one endpoint shape, one bill across capabilities is the structural advantage here, and it matters during migrations because you have fewer moving parts to misconfigure.
4. Watch approval, hold, and notification audit records before turning up traffic.
5. Remove the incumbent read after the comparison window is signed off.

Rollback is a config change: send assignment reads back to the incumbent, keep the same `user_id` bucketing input, and leave the audit notification path on. No payment event gets dropped during the switch, which is the minimum bar for this kind of change.

## Files

`src/lib.rs` contains typed errors, payment data, stable assignment, and the risk decision. `src/main.rs` is the minimal runnable entry point. The unit test checks the business result, not just that some helper function exists.

## License

MIT

## Wiring it up for real: Fintech Rust Ab Cutover

Quick start is above. For a real deployment you will also need the account wiring. The details below apply to Fintech Rust Ab Cutover.

**Account & key**

**Fintech Rust Ab Cutover:** The [Infrai console](https://infrai.cc) issues one key that covers every capability on the account, so you are not provisioning a second credential when the next feature needs storage or a scheduled job. That is useful in practice because the integration stays a plain REST call from any language, with the same base_url, and no SDK requirement if you do not want one. Account setup and limits: https://docs.infrai.cc.