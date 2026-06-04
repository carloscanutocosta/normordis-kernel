//! Drainer supervisionado do outbox de `core-org`.
//!
//! Entrega periodicamente a evidência de auditoria e os eventos de domínio
//! capturados nos outboxes, desacoplando a entrega do caminho de escrita
//! (latência) e garantindo recuperação fiável quando a entrega imediata falha.
//!
//! Em produção, lançar `run_forever` numa tarefa/thread dedicada; monitorizar
//! `DrainStats::pending_*` e `dead_letter_*` (um aumento sinaliza um destino
//! indisponível ou mensagens envenenadas).

use std::time::Duration;

use crate::{
    audit::OrgAuditPort, domain_events::OrgDomainEventPort, error::OrgError, ports::OrgAuditOutbox,
};

/// Estatísticas de uma passagem de drenagem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DrainStats {
    /// Eventos de auditoria entregues nesta passagem.
    pub audit_delivered: usize,
    /// Eventos de domínio entregues nesta passagem.
    pub domain_delivered: usize,
    /// Eventos de auditoria ainda por entregar.
    pub audit_pending: u64,
    /// Eventos de domínio ainda por entregar.
    pub domain_pending: u64,
    /// Eventos de auditoria em dead-letter (esgotaram tentativas).
    pub audit_dead_letter: u64,
    /// Eventos de domínio em dead-letter.
    pub domain_dead_letter: u64,
}

impl DrainStats {
    /// `true` se há mensagens em dead-letter — requer intervenção.
    pub fn has_dead_letters(&self) -> bool {
        self.audit_dead_letter > 0 || self.domain_dead_letter > 0
    }
}

/// Drainer que entrega ambos os outboxes (auditoria + domínio) a partir de um
/// `OrgAuditOutbox`, para os respectivos portos.
pub struct OrgOutboxDrainer<R, A, E>
where
    R: OrgAuditOutbox,
    A: OrgAuditPort,
    E: OrgDomainEventPort,
{
    repo: R,
    audit: A,
    events: E,
}

impl<R, A, E> OrgOutboxDrainer<R, A, E>
where
    R: OrgAuditOutbox,
    A: OrgAuditPort,
    E: OrgDomainEventPort,
{
    pub fn new(repo: R, audit: A, events: E) -> Self {
        Self {
            repo,
            audit,
            events,
        }
    }

    /// Faz uma passagem de drenagem de ambos os outboxes e devolve estatísticas.
    pub fn run_once(&self) -> Result<DrainStats, OrgError> {
        let audit_delivered = self.repo.drain_audit_outbox(&self.audit)?;
        let domain_delivered = self.repo.drain_domain_outbox(&self.events)?;
        Ok(DrainStats {
            audit_delivered,
            domain_delivered,
            audit_pending: self.repo.pending_audit_count()?,
            domain_pending: self.repo.pending_domain_count()?,
            audit_dead_letter: self.repo.dead_letter_audit_count()?,
            domain_dead_letter: self.repo.dead_letter_domain_count()?,
        })
    }

    /// Loop bloqueante: drena, dorme `interval`, repete. Lançar numa thread
    /// dedicada. `on_tick` recebe o resultado de cada passagem (para métricas /
    /// alertas); devolver `false` termina o loop.
    pub fn run_forever<F>(&self, interval: Duration, mut on_tick: F)
    where
        F: FnMut(Result<DrainStats, OrgError>) -> bool,
    {
        loop {
            let keep_going = on_tick(self.run_once());
            if !keep_going {
                break;
            }
            std::thread::sleep(interval);
        }
    }
}
