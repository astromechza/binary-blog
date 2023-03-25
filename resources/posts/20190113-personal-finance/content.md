<meta x-title="Personal Finance Thoughts"/>
<meta x-description="(Imported from old blog)"/>

# Intro

For a few months now I've been trying to find a good home-grown solution to budgetting and tracking my finances.

About a year ago I used `ledger-cli` for a time which is a fairly nice plain-text double-entry accounting system. It worked pretty well and served its purpose but I was always a bit uncomfortable with some of the ways it worked (for example, the custom syntax and very complex accounting rules).

I also want to move towards a solution which supports the following additional things:

- **git-based storage**: So I can use a central storage like Github/Bitbucket that can easily synchronise between devices.

- **separate logic from syntax**: Be able to load transactions from `yaml` or `json`, or anything generated from external tooling.

- **simplify accounting**: Remove concepts of lot-prices, conversion pricing, and a bunch of other things that can be represented in other ways.

- **treat commodities and currencies the same**: Currency values like `£100` can be represented as `100 £` or `100 GBP`.

The implementation would be done as a set of pure-stdlib libraries that can be combined with extensions that provide interactions with other formats, data sources, or analysis.

# Core concepts and rules

- An `amount` is a combination of a mixed-precision decimal combined with a `commodity` like `£`, `$`, `GOOG`.

- An `entry` is a negative or positive `amount` combined with an account. It indicates a change in value of that account.

- A `transaction` is a list of `entries` that balance to `0` across each `commodity`. Combined with a precise date/time and metadata such as labels, notes, etc.

- An `account` can contain any number of child `accounts`. The total value of an `account` is the sum of all `amounts` in that `account` and all child `accounts`. These can be seen as a tree of values.

- An `account` can only be used in an `entry` if it has been "defined". The process of defining an account is fairly flexible and allows you to specify some properties, labels, and description.

# Data sources

By extracting the core concepts from the data source and presentation, we can easily define a core library from which we can load data.

By treating the data source as just a stream of structured documents, wherein each document represents a transaction, account defintion, or other core type; we get an easily abstracted system which can be extended to cover many different use cases:

- Recursive directory structure containing files of structured documents
- A sqlite database on disk
- A remote database such as Postgresql or Amazon DynamoDB
- A document store like ElasticSearch or Redis

# Data presentation

If the core library contains the many concepts that allow access to transactions, accounts, and the sources therein, we can provide an API on top of which various other systems can be built:

- Filtering of transactions and entries by labels, types, etc..
- Analysis of account values over time
- Budgeting and other calculations
- Projections of growth

The API of the core library would be driven by the presentation tools. I'm not going to try and guess at the API at this time, since I'm almost certain I'd get it wrong!

# Design goals

Most of my requirements are routed in making this a nice and extensible system without too many edge cases or branch conditions. I'll try and set up an API that makes it easy to built systems on top of this, rather than trying to include all functionality in one place.

For example, the core API doesn't need to know anything about data sources or file reading, or even how to serialise and deserialise the objects. It only needs to support the API that is exposed.
