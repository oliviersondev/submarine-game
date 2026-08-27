# Documentation de conception

Cette documentation définit la direction produit et technique de *Submarine Game*. Elle distingue le prototype actuellement présent dans le dépôt des fonctionnalités visées.

## Documents

| Document | Objet | Statut |
|---|---|---|
| [Game Design Document](game-design.md) | vision, boucle de jeu, rôles, règles et expérience mobile | brouillon 0.1 |
| [Architecture](architecture.md) | architecture actuelle, cible, scénarios, qualités et risques | brouillon 0.1 |
| [Roadmap](roadmap.md) | vertical slices, livrables et critères de validation | proposée 0.1 |

## Ordre de lecture

1. Lire le GDD pour comprendre le jeu à construire.
2. Lire l'architecture pour comprendre les contraintes et les frontières du système.
3. Utiliser la roadmap pour choisir le prochain incrément de développement.

## Règle de maintenance

- Une fonctionnalité cible n'est pas considérée comme livrée parce qu'elle est documentée.
- Tout changement majeur de règle met à jour le GDD.
- Tout changement de frontière, protocole ou autorité met à jour l'architecture.
- Tout changement de priorité ou de critère de sortie met à jour la roadmap.
- Une décision technique structurante et controversée doit être consignée dans un ADR dédié avant son implémentation.

## Direction retenue

- Univers fictif inspiré de la Seconde Guerre mondiale.
- Simulation poussée avec procédures adaptées au tactile.
- Jeu coopératif de un à cinq joueurs.
- Bots pour les postes vacants.
- Présentation entièrement 2D sur navigateur ordinateur et téléphone.
- Serveur autoritaire et simulation déterministe.

## Questions encore ouvertes

- Nom définitif du jeu et du sous-marin fictif.
- Direction artistique finale.
- Valeurs d'équilibrage après les premiers tests de mission.
- Navigateurs et appareils mobiles constituant la matrice de support officielle.
- Hébergement de la première démonstration publique.
