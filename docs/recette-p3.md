# Recette P3 — Conduite de tir visuelle manuelle

## État de validation

Implémentation du 6 novembre 2026. Les tests automatisés vérifient les règles et les frontières privées. Cette procédure navigateur/téléphone reste à exécuter ; elle ne constitue pas un compte rendu de recette ni une validation d'équilibrage.

## Préparation

- Lancer `make server` et `make client` ; les deux doivent utiliser le protocole v12.
- Créer une salle solo comme Capitaine, les quatre autres postes étant des bots.
- Reprendre la [recette P1](recette-p1.md) pour observer et créer un contact. Le parcours P3 ne nécessite ni piste sonar ni partage sonar ; le bot Sonar peut continuer son activité normale.
- Distance, cap et vitesse du formulaire sont des hypothèses manuelles. Leurs valeurs initiales ne décrivent pas automatiquement la cible.

## Parcours solo

1. Observer une silhouette et utiliser **CREER CONTACT**. Vérifier l'identifiant visuel, le relèvement et l'heure d'observation.
2. Dans **ESTIMATIONS LOCALES**, régler la distance par pas de 100 m, le cap cible par pas de 5° et la vitesse par pas de 1 nœud. Vérifier les bornes de distance (100–30 000 m) et de vitesse (0–40 nœuds), et le passage du cap 355° à 0°.
3. Appuyer sur **APPLIQUER ESTIMATION**. Attendre l'affichage **APPLIQUE**, avec identifiant visuel, révision, hypothèses et observation datée. Aucun message ne confirme que les hypothèses sont justes.
4. Appuyer sur **PREPARER BOT**. Le bouton reste indisponible tant que les estimations locales diffèrent de celles appliquées ou que le contact sélectionné n'est pas celui désigné.
5. Passer sur **CARTE**, puis utiliser **Tube >** jusqu'au tube configuré. Vérifier que la configuration indique **Visuel r…**, et non une piste sonar P… ; le bot doit charger, configurer, inonder et ouvrir sans autorisation automatique du Capitaine humain.
6. Vérifier la solution : relèvement, durée, âge de l'observation et **précision inconnue**. Si le tir est hors secteur, trop profond ou hors autonomie, corriger le positionnement ou les hypothèses, sans attendre une correction automatique.
7. Appuyer sur **Autoriser** sur le tube voulu. Le bot lance selon la procédure existante ; les événements ne confirment un impact que selon les règles d'écoute habituelles.

## Corrections et perte de vue

1. Sur une nouvelle préparation non encore autorisée, modifier les champs locaux sans appliquer : la révision et les réglages appliqués ne changent pas.
2. Appliquer les corrections : la révision augmente ; les anciennes solutions visuelles deviennent indisponibles et les autorisations sont annulées. Donner un nouvel ordre bot, attendre la reconfiguration et autoriser de nouveau.
3. Abaisser le périscope et manœuvrer : les observations ne se rafraîchissent plus, leur âge augmente. Une solution visuelle peut rester calculable depuis les seules hypothèses anciennes.
4. Attendre l'expiration du contact : une désignation déjà appliquée persiste, sans acquisition cachée. Il faut créer un nouveau contact après réobservation pour transmettre une nouvelle désignation ; aucun rattachement automatique n'est attendu.

## Parcours multijoueur

- Avec un Armement humain, appliquer une désignation au périscope : le poste Armement doit la recevoir sans piste sonar préalable. **PREPARER BOT** doit être indisponible et un ordre bot doit être refusé côté serveur.
- L'Armement utilise **Source >** pour choisir **Visuel r…**, charge puis configure un tube, inonde et ouvre. Le Capitaine autorise ; l'Armement appuie sur **Tirer…**, puis **CONFIRMER TIR**.
- Modifier les réglages locaux de l'Armement doit suspendre son tir jusqu'à configuration ou retour aux réglages appliqués.
- Vérifier qu'une nouvelle désignation visuelle n'altère pas les configurations sonar des autres tubes.

## Tactile et vérifications automatisées

- À 360 px, puis sur téléphone réel : sélection, réglages, application, défilement et accès aux commandes d'autorisation sans clavier ni survol.
- Exécuter `make check-all` et `make test`.
- Les tests couvrent l'attaque et la préparation bot sans sonar, le raté sur mauvaise estimation, l'origine et l'heure de mesure, le vieillissement, les permissions/projections, les bornes, les anciennes révisions et le rejeu déterministe.
- Les tests client peuvent être vérifiés à la compilation avec `cargo check -p client --tests --target wasm32-unknown-unknown` ; cela ne les exécute pas.

Le télémètre, l'estimation assistée, la fusion visuel/sonar et la mission complète M6 restent hors périmètre.
