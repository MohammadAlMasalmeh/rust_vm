# Rust Virtual Memory

Virtual memory and Rust seem p cool. So why not try to tackle two eggs with one pan! Working thru this project step by step, trying to learn Rust fundamentals while also becoming the virtualization/memory goat.

## What I'm trying to learn

- I want to learn how OSes handle virtualization from a software perspective. I'm not gonna make any hardware for this project, even the MMU is just a software abstraction of how the real MMU behaves.
- I want to run experiments on which systems benefit from which workloads. When is it better to have huge page sizes, when should we shrink page sizes, when should we have some sort of hybrid system.
- I want to learn how Rust handles its memory management.



## So far

- Created `trace.rs`. This will read through the traces that I make to test the project. Rn, my idea is to have:
  - `R` (read)
  - `W` (write)
  - `S` (context switch to new proc)
  - `E` (exit a process, ping OS to clean up)

Example trace:

```
S 1
R 0x1000
W 0x1004
S 2
R 0x1000
E 1
```



## Design decisions

- **Context Switching:** `S 1` means everything after belongs to proc 1 untill the next switch. Basically how real hardware does it, the OS just loads a new page table root (CR3 on x86).
- **MMU vs OS:** MMU just translates and sets referenced/dirty bits. OS handles switches, exits, page faults, and eviction.
- **Clock:** Exact LRU is too expensive in real hardware. Trying out Clock (thanks Claude for the recommendation). Reference bit that is set to 1 when a page is accessed, and kick out the one with the reference bit set to 0. On each access when memory is full and we encounter a pagefault, scan for empty reference but also set the reference bit of "valid" pages back to 0 to reset the cycle.

## How to run it

For now, run the parser tests with:

```bash
cargo test
```

Gonna wire up main later to actually make this a real system.
<br><br>
This is just what I have so far, more design decisions and everything will be added as I keep working. I hope that this ends up being some sort of huge roadmap that I can remember every step I took to get to the final product.