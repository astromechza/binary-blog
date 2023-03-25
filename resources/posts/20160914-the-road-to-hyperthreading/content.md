<meta x-title="The road to hyperthreading"/>
<meta x-description="(Imported from old blog) A brief overview of how CPU architecture has brought us to hyperthreading via instruction pipelining and superscalar architectures.."/>

-----

### First some history bits

#### 1. Execution units

An execution unit is a subsection of a CPU dedicated to one kind of instruction
processing.

- ALU (arithmetic logic unit, used for integer operations)
- FPU (floating point unit, used for floating point calculations)
- AGU (address generation unit, compute memory addresses)
- several other more specialised components

The CPU dispatcher is used to decode an incoming instruction and send it to the
specific execution unit which is best suited to doing the work.

#### 2. [Instruction pipelining](https://en.wikipedia.org/wiki/Instruction_pipelining)

With multiple execution units available, most of them stand idle while an instruction
is being performed. Why can't we use the idle components to perform operations
if we have upcoming instructions which we can dispatch to them that don't require
the output of other operations? This is *instruction pipelining*. The CPU can decode
and dispatch additional instructions while already dispatched instructions are still busy.

For example:

- first stage extracts an instruction from the cache
- second stage decodes the instruction
- third stage translates memory addresses
- fourth stage executes the instruction
- fifth stage writes the results back to the registers and memory

We can have a piece of work in each of the stages above at any time.

![pipelined instructions](http://www.gamedev.net/uploads/monthly_05_2013/ccs-78358-0-60361400-1367786551.png)

This was first used around 1940 but became very useful in the late 1970's in
supercomputers dedicated to vector and array processing. You can see how needing
to perform the same operation on a number of items can leverage pipelining.

#### 3. [Superscalar](https://en.wikipedia.org/wiki/Superscalar_processor) CPUs

The next step up from pipelining is superscalar architectures. Instead of having
single copies of each execution unit, a superscalar CPU has multiple ALUs, FPUs,
etc. In a simple scalar CPU architecture, the CPU can execute a single instruction
at a time by passing it to the relevant unit.
In a superscalar architecture, the cpu dispatcher reads the next set of
instructions and can dispatch them concurrently to different units as long as
they don't depend on each other. In the worst case, the instructions cannot are
tightly coupled and cannot be run concurrently, but in the best case it can
sustain an execution rate of more than one instruction per cycle. This works
hand-in-hand with pipelining.

Superscalar CPUs were first commercially released around 1988-1990. If you look
at the high level diagrams of the Intel i7 CPUs you'll be able to see the
multiple copies of the execution units. On [this](https://en.wikipedia.org/wiki/Nehalem_(microarchitecture))
page, it states "3 integer ALU, 2 vector ALU and 2 AGU per core".

#### 4. [Hyperthreading](https://en.wikipedia.org/wiki/Hyper-threading)

Hyperthreading first appeared in 2002 on Intel Xeon and Pentium 4 CPUs. It takes
advantage of the superscalar design in order to provide higher performance for
multi-threaded workloads. By duplicating the part of the CPU that stores the
current execution state for a thread, the dispatcher can execute instructions
for either of the threads and it can more effectively take advantage of all of
the execution units.

![Image of multiple architectural states](http://m.eet.com/media/1073565/optimizing_embedded_designs_fig1.jpg)

Normally, the short-term cpu scheduler can only allocate a single thread onto a
cpu at a time. This context switch happens once every time slices, or when specific
interrupts occur (device IO, forks, etc.). If the cpu supports hyperthreading, it can
hold 2 allocations at a time so the scheduler can use a little intelligence to
keep 2 threads allocated to the cpu whenever possible.

----

It's useful to see that there are always tradeoffs to these designs:

- Superscalar work depends on how your instructions take advantage of the
different execution units. If you are only doing floating point operations, it
will be relying heavily on the FPU's without being able to concurrently work on
the ALU.

- Hyperthreaded workloads depend on how CPU-bound the threads are that are
being allocated. If a thread is heavily using the superscalar functionality,
another thread allocated to the CPU will not be able to share the execution
resources effectively.

- In some predictable workloads it may be useful to disable hyperthreading
entirely since the overthread of the additional context switching may actually
degrade performance overall.

CPU's over the years have become very advanced in order to reorder-instructions
and predict and mitigate potential performance stalls.

I should really try and record a video explaining this on paper.. it's a lot
easier that way..

### What this comes down to:

*Hyperthreading is a marketing misnomer* - kinda. It's all too common now for
CPUs to be advertised as quad core when they only have 2 physical hyperthreaded
computation cores. It is also **extremely** rare to have a workload that gets
the same performance on 2 physical cores vs 1 hyperthreaded core. If you want
brute force concurrent processing power you want multiple cores, while if you
have many light weight threaded workloads, hyperthreading will allow you to more
efficiently use the resources you have.

So at the moment an Intel i5 CPU is usually cheaper than an i7 since it either
has 4 physical cores without hyperthreading or 2 physical cores *with*
hyperthreading, while i7 CPUs commonly have 4 or even 6 cores along with hyperthreading.

Sadly, my Macbook Pro has the i7-4558U which despite a great clock speed and
cache size, only has 2 cores with hyperthreading.

### Impact on cloud VM providers

Cloud providers offering VM (virtual machine)'s, commonly use Intel Xeon processors.
A recently released Xeon processor offers 18 cores with hyperthreading for a total of 36
"threads" or virtual cores. In the operating system each of these 36 threads
can be allocated work so each one appears as a distinct processor when you run
`cat /proc/cpuinfo` or similar. When launching VM's it's important to realise
that unless you are strictly pairing the hyperthreaded cpus together, you may
have 2 customers that share a single physical cpu between their VMs which means
that a workload in one VM may impact the other VM.


#### Other useful links that may explain things better

- [http://www.tutorialspoint.com/operating_system/os_process_scheduling.htm](http://www.tutorialspoint.com/operating_system/os_process_scheduling.htm)
- [https://www.percona.com/blog/2015/01/15/hyper-threading-double-cpu-throughput](https://www.percona.com/blog/2015/01/15/hyper-threading-double-cpu-throughput)
- [https://bitsum.com/pl_when_hyperthreading_hurts.php](https://bitsum.com/pl_when_hyperthreading_hurts.php)
- [http://www.gamedev.net/page/resources/technical/general-programming/a-journey-through-the-cpu-pipeline-r3115](http://www.gamedev.net/page/resources/_/technical/general-programming/a-journey-through-the-cpu-pipeline-r3115)
