# ARKHE Clinical

ARKHE Clinical é uma plataforma desenvolvida em Rust para gerenciar e executar ensaios clínicos com foco na Doença de Creutzfeldt-Jakob (CJD) de maneira eficiente e rigorosa em termos de regulamentação. O projeto adere aos mais estritos padrões da indústria (ICH-GCP, 21 CFR Part 11, e GDPR) proporcionando segurança, rastreabilidade, imutabilidade e análise de dados em tempo real.

## 🚀 Funcionalidades da Plataforma

A plataforma conta com diversos módulos essenciais para a condução de ensaios clínicos robustos:

- **Audit Trail (`audit`):** Trilhas de auditoria imutáveis com registros de hashes em cadeia, e possibilidade de uso de assinaturas digitais (compatível com 21 CFR Part 11).
- **Gestão de Ensaios Clínicos (`trial`):** Controle completo sobre ensaios, definição de fases, critérios de elegibilidade e acompanhamento do status e desfecho dos pacientes.
- **Biomarcadores (`biomarker`):** Análise para o acompanhamento da progressão com foco nos resultados de RT-QuIC.
- **Relatório de Segurança - SAEs (`safety`):** Gestão rigorosa de Eventos Adversos (AE) e Eventos Adversos Graves (SAEs) seguindo templates de preenchimento como o CIOMS I.
- **Consentimento Informado (`consent`):** Rastreabilidade total sobre versões de protocolos, hashes dos documentos, revogação e checagem de assinatura de consentimento dos pacientes, adequando-se ao ICH E6(R2) §4.8.
- **Outros módulos:** Invariantes Regulatórios, Data Lock, Randomização Estratificada, Pseudonimização e Anonimização para exportação de dados, todos com forte aderência aos padrões de conformidade.

## 📦 Como Compilar e Rodar os Testes

Este projeto é desenvolvido com **Rust** e as suas ferramentas (`cargo`). Para instalar e executar o projeto e seus testes, siga os passos abaixo:

1. Acesse o diretório da crate:
   ```bash
   cd crates/arkhe-clinical
   ```

2. Para compilar o projeto:
   ```bash
   cargo build
   ```

3. Para executar os testes e garantir a funcionalidade de todos os módulos:
   ```bash
   cargo test
   ```

---

## 📖 Tutorial Rápido de Uso

Abaixo apresentamos como utilizar alguns dos principais componentes da plataforma através de código.

### 1. Inicializando o Audit Trail

O **Audit Trail** mantém registro imutável das ações na plataforma. Veja como você pode configurá-lo e registrar uma ação:

```rust
use arkhe_clinical::audit::{AuditTrail, AuditAction, AuditSeverity};
use tokio;

#[tokio::main]
async fn main() {
    // 1. Criar uma nova instância de Audit Trail
    let audit_trail = AuditTrail::new();

    // 2. Adicionar uma nova entrada
    let entry = audit_trail.append(
        "user_123",                    // user_id
        "Investigator",                // user_role
        AuditAction::TrialCreated,     // Ação
        AuditSeverity::Info,           // Severidade
        "Novo ensaio clínico criado."  // Descrição
    ).await;

    match entry {
        Ok(audit_entry) => println!("Entrada de auditoria criada com sucesso. Hash: {}", audit_entry.hash),
        Err(e) => eprintln!("Falha ao criar o log: {}", e),
    }
}
```

### 2. Gerenciando Eventos Adversos de Segurança

O módulo **SafetyMonitor** permite relatar e buscar Eventos Adversos, além de gerar automaticamente relatórios como o CIOMS I.

```rust
use arkhe_clinical::safety::{SafetyMonitor, AdverseEvent, AeSeverity, Causality, AeOutcome, SaeCriterion};
use arkhe_clinical::ClinicalId;
use chrono::Utc;

fn report_safety_event() {
    let mut safety_monitor = SafetyMonitor::new();

    let onset = Utc::now();
    let ae = AdverseEvent {
        id: ClinicalId::new(),
        subject_code: "SUBJ-001".to_string(),
        description: "Reação alérgica aguda".to_string(),
        onset_date: onset,
        stop_date: None,
        severity: AeSeverity::Severe,
        causality: Causality::Possible,
        expected: false,
        outcome: AeOutcome::NotRecovered,
        sae: true,
        sae_criteria: vec![SaeCriterion::LifeThreatening],
        action_taken: vec![],
        reported_to_sponsor: None,
        reported_to_regulatory: None,
        cioms_form: None,
    };

    safety_monitor.report_ae(ae.clone());

    // Gera sumário
    let summary = safety_monitor.safety_summary();
    println!("Total de SAEs: {}", summary.total_saes);

    // Gera formulário CIOMS I automaticamente a partir do evento
    let cioms = safety_monitor.generate_cioms(&ae);
    println!("CIOMS Report ID: {}", cioms.report_id);
}
```

### 3. Registro de Consentimento Informado

Para manter-se compatível com regras de compliance de saúde, o gerenciamento de consentimentos (TCLE) é estrito e requer hashes do texto:

```rust
use arkhe_clinical::consent::{ConsentRegistry, ConsentDocument, SignatoryRole};

fn manage_consent() {
    let mut registry = ConsentRegistry::new();

    // 1. Registrar um novo documento (e gerar seu hash internamente)
    let protocol_text = "Eu concordo em participar deste ensaio clínico...";
    let doc = ConsentDocument::new("v1.0", "pt-BR", protocol_text, "IRB-12345");
    registry.register_document(doc.clone());

    // 2. Criar um consentimento assinado para um participante
    let consent = registry.create_consent(
        "SUBJ-001",
        &doc,
        "João da Silva",
        SignatoryRole::Participant,
        "INV-999",
        "SITE-A",
    );

    // 3. Checar a validade e a integridade da assinatura
    if registry.is_valid("SUBJ-001") {
        println!("O paciente tem um consentimento válido e ativo.");
    }

    let is_intact = registry.verify_integrity(&consent, &doc);
    assert!(is_intact, "O texto do protocolo assinado difere do atual!");
}
```

## Arquitetura e Bibliotecas Externas
Este projeto usa:
- **Serde:** para serialização (JSON, YAML).
- **Tokio:** para runtime assíncrona.
- **Chrono & Uuid:** para timestamps em logs e identificadores precisos.
- **Sha2:** na verificação de integridade e registro de audit trail.

---

O sistema assegura conformidade robusta focada em trilhas de dados criptográficas, integridade na submissão de relatórios e preservação do direito à privacidade (GDPR) durante os ensaios clínicos.
