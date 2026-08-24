# 🏛️ RDG/MFE/Q — Extensão Clínica (ARKHE Clinical) v1.4.0

**Versão:** 1.4.0
**Data:** 2026-08-24
**Status:** ✅ Especificação formal honesta

## Princípio Orientador

> **Use Lean 4 para o que ele é bom: verificar estruturas de dados, invariantes de tipo e funções totais. Deixe propriedades criptográficas, operacionais e regulatórias para a implementação Rust + processos de auditoria.**

## O que está formalizado

| Tipo/Função | Descrição | Status |
|-------------|-----------|--------|
| `TrialPhase` | 4 fases de ensaio clínico (ICH-GCP) | ✅ `Fintype`, `DecidableEq` |
| `AeSeverity` | 5 níveis de severidade (CTCAE v5.0) | ✅ `Fintype`, `DecidableEq` |
| `SaeCriterion` | 6 critérios de SAE (ICH-GCP E2A) | ✅ `Fintype`, `DecidableEq` |
| `AeSeverity.ordinal` | Mapeamento para ℕ | ✅ Função total |
| `ordinal_injective` | Prova de injetividade | ✅ Teorema verificado |
| `AdverseEvent` | Estrutura de dados com campos tipados | ✅ `structure` |
| `Biomarker` | Série temporal enumerável | ✅ `Streamable` |
| `RTQuIC` | Biomarcador específico | ✅ Instância de `Biomarker ℕ` |
| `Admissible` instances | Codificação para sistemas externos | ✅ Para todos os tipos |

## O que NÃO está formalizado (e por quê)

| Conceito | Motivo |
|----------|--------|
| Unicidade de hash SHA-256 | SHA-256 não é injetiva (pigeonhole: ℕ → Fin(2²⁵⁶)) |
| Imutabilidade de audit trail | Propriedade de SISTEMA (WORM storage, append-only log) |
| Bijeção SAE ↔ CIOMS I | Relação é muitos-para-um na prática clínica |
| Pseudonimização injetiva | GDPR exige resistência a re-identificação, não injetividade |
| Validade de consentimento | Processo de assinatura/data/versão, não propriedade de tipo |

## Ferramentas de Integração Recomendadas

### 1. Aeneas + Charon (Extração Rust → Lean 4)

**Status:** Maduro, uso industrial (Microsoft SymCrypt/ML-KEM)
**Pipeline:** `Rust → Charon (LLBC) → Aeneas → Lean 4 (puro funcional)`
**Monads:** `Result α` (sucesso/pânico/não-terminação)
**Limitações:**
- Generic functions com trait bounds precisam de monomorfização manual
- External crates não são extraíveis (Charon não vê MIR além do workspace)
- `while` loops viram recursão com obrigação de terminação explícita

**Referência:** cite🛠web_search:37#11:~:text=Charon lowers Rust programs to a clean...typed intermediate representation

### 2. Hax (Extração Anotada Rust → Lean 4)

**Status:** ~10–15 meses, backend experimental
**Pipeline:** `Rust + #[hax::contract] → cargo hax into lean`
**Monads:** `RustM` (overflow/panic paths explícitos)
**Limitações:**
- "Can only translate a fragment of Rust" (palavras dos autores)
- "Verification Facade" (ePrint 2026/670): 3 classes de semantic gap, 5 PoC exploits
- TCB = 35 fases OCaml, 113 `assume val`

**Referência:** cite🛠web_search:37#2:~:text=hax's Lean backend is ~1 year old...experimental by its own manual

### 3. Verus (Verificação Nativa em Rust)

**Status:** State-of-the-art, SMT-based (Z3)
**Abordagem:** Especificações dentro do Rust (`requires`, `ensures`, `invariants`, ghost state)
**Backend:** SMT solver (Z3) — reduz necessidade de provas manuais
**Uso:** Verificação de estruturas de dados complexas e algoritmos

**Referência:** cite🛠web_search:37#1:~:text=Verus is a state-of-the-art Rust-native verifier...SMT solvers

### 4. lean-rs (FFI Rust → Lean 4)

**Status:** Crate publicado, suporte a Lean 4.30.0–4.34.0-rc2
**Features:**
- RAII refcounting para referências `LeanOwned`
- Lifetime bounds para `LeanBorrowed`
- Thread-safe shared references (`LeanShared`)
- `lean_inductive!` macro para tipos Lean
- `Nat` conversions via `num-bigint`

**Uso:** Chamar funções Lean verificadas a partir do Rust

**Referência:** cite🛠web_search:37#5:~:text=lean-rs is the typed FFI binding...to the Lean 4 runtime

### 5. Lean Squad (AI Agentic Workflow)

**Status:** Public, GitHub Next (Don Syme)
**Resultados:** 1,220 teoremas provados em raft-rs/quiche/PX4 por ~$7/run
**Close rate:** ~30% por passagem agentic (não 99% dos benchmarks olímpicos)
**Uso:** Fechar `sorry`s automaticamente em CI/CD

**Referência:** cite🛠web_search:37#2:~:text=Lean Squad (Don Syme / GitHub Next)...1,220 theorems across raft-rs/quiche/PX4

## Arquitetura de 3 Camadas

```
┌─────────────────────────────────────────────────────────────┐
│  CAMADA 3: Implementação Rust (ARKHE Clinical)             │
│  - Audit Trail (append-only log com SHA-256)               │
│  - Consent Management (assinaturas digitais)               │
│  - Safety Monitoring (SAEs, CIOMS I)                      │
│  - Data Lock (estado operacional WORM)                    │
├─────────────────────────────────────────────────────────────┤
│  CAMADA 2: Especificação e Verificação em Lean 4           │
│  - Estruturas de dados (TrialPhase, AeSeverity, etc.)     │
│  - Invariantes de tipo (ex: patient_id > 0)               │
│  - Funções totais (severity_ordinal, toCiomsI)            │
│  - Extração via Aeneas/Hax ou FFI via lean-rs             │
├─────────────────────────────────────────────────────────────┤
│  CAMADA 1: Fundação Matemática (RDG/MFE/Q)                │
│  - Sₙ, Streamable, SID, Finite, Admissible                │
│  - Metateoremas (cardinalidade, fechamentos)              │
└─────────────────────────────────────────────────────────────┘
```

## Roadmap de Integração

| Fase | Tarefa | Ferramenta | Tempo Estimado |
|------|--------|------------|----------------|
| 1 | Compilar `Clinical.lean` com `lake build` | Lean 4 + Mathlib | 2h |
| 2 | Extrair funções críticas do Rust (e.g., `severity_ordinal`) | Aeneas/Charon | 1–2 dias |
| 3 | Provar equivalência entre extração e especificação Lean | Lean 4 tactics | 2–3 dias |
| 4 | Configurar CI/CD com `lake build` + `#print axioms` | GitHub Actions | 4h |
| 5 | Integrar FFI lean-rs para chamada de validadores Lean do Rust | lean-rs | 1–2 dias |
| 6 | Adicionar Lean Squad para fechamento automático de `sorry`s | GitHub Next | 1 dia |

## Compilação

```bash
cd RDG-MFE-Q-Lean4
lake update
lake build
lake exe sanity
```

## Licença

MIT — ARKHE Project
