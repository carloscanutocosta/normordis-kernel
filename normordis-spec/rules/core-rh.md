# Regras de Negócio — core-rh

Estas regras definem identidade, perfis funcionais e catálogo de roles. O core-rh
não guarda passwords, tokens ou segredos no modelo público.

---

## UserProfile

### RH-R01 — username único

`username` deve ser único no sistema (validação ao nível de repositório).

### RH-R02 — email válido

`email`, se presente, deve ser endereço RFC 5321 válido.

---

## PersonAssignment

### RH-R03 — valid_until posterior a valid_from

`valid_until`, se presente, deve ser estritamente posterior a `valid_from`.
Implementações devem rejeitar ranges invertidos mesmo que o schema aceite
(invariante de camada 3).

### RH-R04 — Sem afetações activas sobrepostas

Uma pessoa não pode ter duas afetações activas para o mesmo `position_id`
em períodos sobrepostos. Regra verificada pelo serviço; requer contexto de repositório.

---

## Role

### RH-R05 — RoleId sem espaços

`RoleId` não pode conter espaços (invariante do modelo).

### RH-R06 — RoleId não vazio

`RoleId` não pode estar vazio após trim.

### RH-R07 — name obrigatório

`name` é obrigatório e não pode ser apenas whitespace.

### RH-R08 — Roles inactivos não são atribuídos

Roles com `is_active = false` não devem ser atribuídos a novos utilizadores.
Regra verificada pelo serviço; requer contexto de múltiplos registos.
