<meta x-title="Thoughts on a fault injection API"/>
<meta x-description="(Imported from old blog) Managed fault injection APIs for providing powerful chaos engineering functionality. How about a /faultz endpoint for in-process chaos engineering?"/>

I recently re-watched Kelsey Hightower’s ["Monitoring from the inside"](https://vimeo.com/173610242) (2016) talk regarding leveraging healthcheck endpoints for readiness monitoring of services in a Kubernetes context and it set some seeds going in my mind. I’ve been using `/healthcheck` or `/healthz` endpoints for a while now, using them to debug issues or just view service specific facts while a service runs. In the same team, we’ve also been making a push towards various deep integration tests to increase code coverage inside various asynchronous workflows during CI/CD periods. One of the things we needed was a way to trigger failures in these workflows **on purpose**. _We’d rather the workflow ended gracefully with an explicit failure message rather than causing uncontrolled failure of other unrelated systems or workflows._

This is harder than you want it to be, as you’re fighting against the flow of other devs fixing the very classes or issues you’re trying to trigger!

Part of this is just picking the faults correctly, there’s no point trying to trigger "fixable" issues. A syntax bug will be fixed shortly, while a complex behaviour in network-partition scenarios would be a better use of your time to test and is something that is likely to happen due to more uncontrollable issues like hardware failure, denial-of-service, resource starvation, etc.

Our first and minimal version of this is to include hidden and feature flagged options in a workflow which can sit dormant until a specific point in the workflow in which it can be triggered.

An equivalent of:

```python
if fault_injection_is_enabled and workflow._stage_N_fault:
    raise Exception()
```

This worked, as we could run failure tests as part of our CI tests and we could ensure that we didn’t have a regression in the behaviour of such a failure. But it always felt fairly messy and having “hidden” bits of your API is just one step closer to a backdoor.

**However**, I’d like to investigate taking this a bit further.

In many existing “Chaos Engineering” techniques, most injected failures are at the OS level:

- iptables can cause network partitions
- bash scripts can be used to consume resources, fire requests against things
- etc..

What about having service owners commit to providing some service-level fault capabilities?

Note: these are just brain-farts, and don’t include serious thought regarding authorization, limitations, deadman-switches, etc..

### Example 1: Trigger a panic in the incoming request’s context

Pretty brute force and in-elegant, but why not? Rather than attempting to ensure that the service will never panic (100% coverage?), why not ensure that if it does panic, it won’t affect the larger system other than maybe a 500 response. (You could also recover from it, but that’s not the point here)

```
POST /faultz/trigger/panic
```

Expected results: process dies, socket closes, LB doesn’t send any more traffic to it, HA kicks in and launches it somewhere else, etc..

This is pretty similar to a Chaos or QA team just kill-9'ing your process.

### Example 2: Slow down the next N incoming requests by a random interval in 0-M

Attempt to simulate load on the system causing requests to be processed more slowly. This would need to be bolted in at a higher level in front of your http handlers so that it could be generalised nicely between them. It would also be wise to be able to cancel this behaviour in case you’re testing in prod.

```
POST /faultz/start/slow-period/00001
api: /my/target/path 
remaining_count: 150
delay_ms_min: 0
delay_ms_max: 60000
expire_at: 2018-01-01T05:56:57 
```

Now hopefully you’ll see the next 150 requests to `/my/target/path` take longer than expected, but this behaviour will end after either 150 requests, or 60 seconds.

### Example 3: decrease size of the apps connection pool dynamically

This could be done by just loading it with more requests, but perhaps it could be achieved internally as well by decreasing the number of requests the service will handle?

This could also be achieved by example 2.

### Example 4: queue up a failure for the next time a particular code-point it hit when certain parameters are matched

```
POST /faultz/prepare/database-get-failure/0001
model: User
failure: http-500
expire_at: 2018-01-01T05:56:57
requirements:
  - username: BillGates
```

This kind of equivalent to my previous example of fault injection in a team of mine.

### Example 5: gradually consume more and more RAM over the next N seconds

```
POST /faultz/start/ram-consumer/0001
target: 1GB
allocation_randomness: 0.7
expire_at: 2018-01-01T05:56:57
```

This may just begin allocating a large linked list of arrays of random int64’s at a fairly controlled-or-variable rate. Perhaps we expect to see alarms go off, GC to increase, or other failures.

### Example 5: allow the faultz api to be self documenting

Perhaps `GET /faultz/help` can list the available failure API endpoints in a swagger-ish form?

Unsure if this is such a good idea :) but could be interesting for teams that want to own some of their own chaos engineering or fault injection.
