<meta x-title="Coordinated Omission in load measurements"/>
<meta x-description="(Imported from old blog)"/>

Many people have written about "Coordinated Omission" before. The talks and posts that first made me aware (and helped me to understand) the issue were:

- ["How NOT to measure latency"](https://www.youtube.com/watch?v=lJ8ydIuPFeU)
- ["You load generator is probably lying to you"](http://highscalability.com/blog/2015/10/5/your-load-generator-is-probably-lying-to-you-take-the-red-pi.html)

This post is mostly a small attempt for me to rationalise and underline my own understanding and hopefully provide others with a quick reference when discussing the issue.

## A quick analogy

![Image of people queuing](coffee-queue.jpg)

Imagine you owned a coffee shop and wanted to determine whether a planned staffing, procedure, or layout change was worth it. Naturally you'd want to gather a bus full of sample participants and have them all go through your shop in both of the configurations over the course of a few hours or a day. To keep the tests consistent, you want to send people in at a fixed rate and have them order the same distributions of things.

How do you measure the time taken for the interaction?

- The end of each participants session is easy to determine: when they leave the shop through the door.
- The beginning of each session is a bit more interesting:
  - If it's the time they first reach the counter, then you're only measuring the individual interaction. You can't really aggregate these very well as they lose the context of the other requests. They don't include queue time and won't really show the effect of introducing more counters or servers in the shop.
  - If it's the time they first hit the queue, then you're losing time introduced by the fixed rate. The queue can spike up or drain dramatically if the staff changes while processing the queue.
  - If it's the time you release them from the bus outside at the fixed rate, then hopefully it doesn't include too much noise from participants taking different routes, or walking at different speeds.
  
So in the end, you _have_ to start measuring from the moment you release the person, having minimised any noise you can. 

If the queue is so long, that it reaches all the way out of the shop and to your bus, you **cannot** just pause each participants session. You have to give them a ticket with their start time even if they haven't left the bus yet.

## Coordinated Omission

This problem is called Coordinated Omission. Coordinated because the client, which is meant to be malicious, ends up coordinating with the server through the latency of each interaction. As latencies go up, the client ends up sending requests less frequently and effectively omitting timing and data that should have been included in the measurement.

## Load testers often face the same problem

Most load testers begin one or more units of computation (thread, goroutine, etc) and have each unit send a number of blocking requests at a given rate.

This is all fine and good, until requests start taking more time than the desired interval. Then we face the same coffee shop problem. 

To get around this, load testers like [wrk2](https://github.com/giltene/wrk2) must begin timing each request according to a "plan" or "schedule" regardless of whether the request has been sent or not. 

It's interesting to note that if you are able to virtualise the connection stack, and track responses asynchronously or use non-blocking requests, you don't have to get around this issue, although you do have to take the memory hit or storing the inflight requests (this may also be a more accurate testing framework).
