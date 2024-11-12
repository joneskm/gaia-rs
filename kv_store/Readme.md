# Key - value storage for Gears 

## Purpose

This crate implements key-value storages with caching on top of 
IAVL Tree from [tree](../trees/Readme.md) crate and ensure data safety
by prefixing stores.

## Diagram

To help understanding of this crate look at this diagram which shows most important relation on this crate.
I omitted details abouts fields or all traits which being implemented instead this shows composition of 
structures to help understanding of purpose for each structure.

![relation of store structures](./assets/store_relation.svg)

## Details

### Color marks

Let's explain a few moments.

**Yellow** items is external or from dependencies. 
For example `DB` is generic used in all structures 
and has bound to `Database` trait from `database` crate.

**Purple** items should be used only in `gears` and ideally 
this types hidden from users(other developers). 
It contains methods which could break consistency of any application.

**Green** items is public items which user will interact with help of
application context.

### Gas

This crate doesn't implement gas metering, so every operation is infallible.

### KV Bank

There is 3 variants for kv bank:
- ApplicationKVBank - application wide bank/store;
- TransactionKVBank - store for usage during transaction processing;
- QueryKVStore - readonly store for queries at specific height.

#### ApplicationKVBank

So, what application wide mean? Mainly this is store which used during 
`{begin/end}_block`, `init_genesis` and used during `commit` for committing 
changes. It has cache for deletion/insertion of values(range iterates over cached values),
but single layer of cache(more details in `TransactionKVBank`).

#### TransactionKVBank

This store used for processing transactions. Its state should be drained 
by `ApplicationKVBank`, but only for valid transaction. All other changes should be discarded.

For this reason `TransactionKVBank` has 2 layers of cache for `tx` and `block`. 
Values in `tx` overwrite all values from `block` and persistent storage during iteration,
get, delete. When tx successfully executed `tx` cache gets *upgraded* to `block` layer 
in case tx fails all `tx` cache should be cleared.

#### QueryKVStore

There nothing special about this store. Use for reading data from persistent storage.

_Note:_ you can't query data from head version, meaning data should be `commit`'ed.

### Multi Bank

It has same 3 variants as kv banks, but let's focus on purpose of this structure.

There is difference in implementation. While `ApplicationKVBank` and `TransactionKVBank`
different types this structures uses type state pattern with `MultiBankBackend` trait
to help access to different kv store variants. 

This store ensure that there no overlap between different *store keys*
so you can't overwrite data in store B using store A 'cause multi bank
would prefix your `DB` with prefix and wrap `DB` with `PrefixDB<DB>`.
This mean that `*MultiBank` structure contains hash map with `PrefixDB<DB>` 
instead of `DB`.

### KVStoreBackend{Mut} & MultiStoreBackend{Mut}

This is not public enum which collects all possible variants of stores 
in this crate to provide them as backend for public stores.