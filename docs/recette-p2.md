# Recette P2 — Détection du périscope

## État de validation

Implémentation du 5 novembre 2026. Les tests automatisés couvrent les règles et les frontières privées. Cette procédure manuelle reste à exécuter ; elle ne constitue pas un compte rendu de recette ni une validation d'équilibrage.

## Préparation

- Lancer `make server` et `make client`, avec client et serveur au protocole v11.
- Créer une salle solo comme Capitaine ; les quatre autres postes sont des bots.
- Reprendre les contrôles de la [recette P1](recette-p1.md) pour orienter le périscope et commander le bot Pilote.
- Aucune alerte d'exposition ni confirmation de détection n'est attendue. Les pings ennemis entendus ne prouvent pas une acquisition visuelle.

## Parcours joueur

1. Rejoindre 12 m et une faible vitesse, observer brièvement le convoi puis abaisser le périscope. Vérifier que les contrôles et contacts P1 restent fonctionnels, sans nouveau bandeau de risque.
2. Approcher le convoi et conserver le périscope levé. À moins de 3 km, l'escorte peut acquérir une observation après exposition ; les cargos le peuvent à moins de 2 km. Un rapprochement ou une vitesse plus forte raccourcit l'acquisition. Observer les réactions éventuelles sans attendre de confirmation de détection.
3. Abaisser le périscope et changer de route. L'escorte ne doit pas suivre instantanément chaque manœuvre : elle conserve une estimation ancienne, potentiellement rafraîchie par ses moyens acoustiques.
4. Répéter des observations très brèves avec de courtes interruptions : ne pas supposer que chaque abaissement annule le risque déjà accumulé.
5. Dépasser 14 m ou 6 nœuds, vérifier l'abaissement automatique existant et l'absence de remontée automatique au retour dans les limites.
6. Vérifier le parcours à 360 px : aucun contrôle supplémentaire ni message d'avertissement n'est ajouté.

## Références automatisées

Exécuter `make check-all` et `make test`.

- `visual_detection::tests` : progression, accélération par proximité/vitesse, cargos moins efficaces, portée inclusive, erreur d'observation, oubli progressif, valeurs invalides et répétabilité.
- `world::tests` : transmission des cargos avec escorte hors portée, exclusion des bâtiments coulés, absence d'observation périscope rentré même en surface, vieillissement et indépendance aux déplacements cachés après perte de vue.
- `periscope_visual_detection_respects_automatic_lowering_before_sampling` : branchement de la perception dans le tick autoritaire et respect des limites avant observation.
- Fixtures du protocole v11 et tests existants de permissions/projections des rôles.

Les propriétés privées (absence d'omniscience, transmission cargo et erreur de mesure) sont vérifiées par tests, pas déduites d'un ping entendu dans le navigateur. La détection de la coque en surface périscope rentré reste hors P2.
