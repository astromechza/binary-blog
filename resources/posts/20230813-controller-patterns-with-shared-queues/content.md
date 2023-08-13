<meta x-title="Controller patterns with shared queues"/>

I've spent a lot of time in the last few years working on distributed systems that follow a particular pattern, what I like to call queue-based controllers. These aren't particularly novel or new (they're the core of how Kubernetes and many other related systems work) but each time I apply them I end up learning more and improving my understanding of how to make them more scalable or reliable. In my current role with Humanitec I've been applying the pattern with a persistent and durable RabbitMQ queue and I'd like to use this post to share some of the learnings here.

## 🤔: But wait, what are these controllers we're talking about?

If you imagine that an item being processed is a little state machine, then a "controller" runs a control loop repeatedly on that item. The purpose of the control loop is to converge the little machine toward its desired state.

The nice advantage of composing an asynchronous system in this way is that you gain great fault-tolerance and fault-recovery properties since each execution of the control loop must result in either no action (the item already being in its desired state) or some operation being triggered to slowly move towards the desired state or back off and wait until it is safe to do so.

Usually, a system is made up of a fairly complex web of different controllers each operating on a particular aspect of the item being processed - the smaller and more focused you can make a controller, the more likely it is that some subset of control loops can make progress even while others may be under a fault condition.

Kubernetes is a good example here, as its many controllers each operate on particular aspects of the API resources. For example, a replica-set-controller repeatedly ensures that the correct number of Pods exist with the correct configuration and listens for any events that may require it to run again.

## Scaling event-based control loops

In traditional control loops, we usually 


https://www.notion.so/benmeier/Blog-Post-on-controller-patterns-with-an-external-queue-af8cfab9efe34a2681e8cab33c53cc68?pvs=4