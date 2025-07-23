# AI models used and prompts

The AI tools used were ChatGPT-4o, Gemini, and Claude 4 via desktop chat
interfaces. While they were helpful in answering isolated questions, their
support was generally shallow when it came to deeper Rust
internals—particularly around Tokio and async behavior, which acted like a
black box. As expected, AI struggled to identify runtime-specific issues unless
explicitly pointed toward them. In practice, manual debugging and direct
consultation of documentation (e.g., reqwest, tokio, criterion) were essential
and far more effective.

Most prompts focused on research into Merkle trees and explored conceptual
parallels like Git DAGs, how Ethereum structures them, and ideas for scalable
implementations (e.g., rollups using max-sized Merkle trees). These were more
exploratory and architectural in nature.

---
## Code-related prompts and outcomes:
1. "As a Rust expert..."
used to set context and generate an initial code structure.

2. "How to fix the test..."
yielded minor suggestions, but the core issue (blocking IO when retrieving
Merkle root and managing multiple reqwest clients) needed a manual fix.

3.	"Help me add a verify_proof function for a Merkle tree in Rust"
guided the addition of structural validation, hash computation, and hex decoding.

4.	"Write tests for Merkle proof verification covering edge cases"
prompted a solid test suite.

5.	"Why is Docker failing to build the rust app? with the following errors..."
asked repeatedly but was unproductive; ultimately resolved by referencing
a working Dockerfile from a prior project and adapting it manually.
---

Smaller prompts covered documentation templates and deployment tooling. While
some deployment options looked promising, they were either not free or too slow
for quick iteration, so deployment was handled using a familiar system.

Overall, the AI acted more as a sanity checker and assistant for known problems
than a source of deep insights or robust solutions.
