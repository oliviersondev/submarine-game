# Recette P4 — Courses de torpilles et explosions

## État de validation

Implémentation du 9 novembre 2026. Les règles autoritaires, permissions et données réseau sont testées automatiquement. La recette navigateur/téléphone ci-dessous reste à exécuter ; aucun résultat audiovisuel réel n'est déclaré validé par ce document.

## Préparation

- Lancer `make server` et `make client`, tous deux au protocole v13.
- Créer une salle solo Capitaine avec quatre bots. Préparer une attaque via la [recette P3](recette-p3.md) ou la procédure sonar existante.
- Commencer à volume modéré. Vérifier **SON ON**, les réglages sonores du navigateur et l'absence de mise en sourdine de l'onglet. Les navigateurs peuvent exiger une interaction avant d'autoriser l'audio.

## Carte et course estimée

1. Après configuration d'un tube, ouvrir la carte et sélectionner ce tube : vérifier les pointillés et le signe **+** d'interception supposée. Aucun point ne doit représenter une position ennemie réelle.
2. Changer de tube : la prévision doit correspondre à sa configuration appliquée, pas aux seuls réglages locaux en cours d'édition. Un tube sans solution ne montre pas de prévision.
3. Autoriser et lancer : vérifier le symbole **o TOR…**, la courte traînée et les délais/autonomies du poste Armement/Capitaine. La carte est orientée au nord, rayon 10 km.
4. Effectuer plusieurs lancements : les identifiants et courses restent distincts, même si des trajectoires se superposent. Un délai d'interception nul n'affiche pas automatiquement un succès.
5. Sur un raté ou une fin non observée, attendre la fin de course estimée : aucun marqueur précis d'impact ni confirmation de naufrage ne doit apparaître.

## Explosion entendue

1. Sur un impact à portée d'écoute, vérifier le son sourd, le bandeau et l'entrée dans l'historique. Aucun point d'explosion précis ne doit être ajouté sur la carte.
2. Passer sur **MUET** : les sons actifs s'arrêtent ; bandeau et historique restent disponibles lors d'une nouvelle explosion.
3. Réactiver le son : les événements précédents ne doivent pas être rejoués.
4. Avec plusieurs explosions rapprochées, vérifier l'absence de saturation audio et la conservation des événements distincts. Un seul son peut représenter un groupe d'explosions.
5. Vérifier aussi une explosion de charge entendue : le son n'est pas une confirmation d'impact d'une torpille sur un cargo.

## Gerbes au périscope

1. Avant l'impact, maintenir le périscope levé, dans les limites d'utilisation, et observer la direction de la cible. Vérifier une gerbe temporaire au bon relèvement, s'estompant en deux secondes.
2. Ne pas créer de contact visuel : une explosion reste observable sans fiche de contact préalable.
3. Observer un impact avec le périscope abaissé ou orienté ailleurs, puis relever/réorienter : aucune gerbe manquée ne doit être rejouée. L'écoute peut néanmoins avoir produit son propre retour.
4. Fermer la vue ou abaisser le périscope pendant une gerbe, puis rouvrir : l'effet ne doit pas réapparaître depuis l'historique.
5. Sur un impact visible au-delà de 6 km mais dans la portée du périscope (8 km par défaut), une gerbe peut être vue sans son. La course de torpille reste estimée ; aucune association automatique ne confirme quelle torpille a touché.
6. Une gerbe ne doit annoncer ni l'identité du bateau ni sa destruction. Deux gerbes simultanées peuvent être affichées séparément.

## Multijoueur et tactile

- Le Capitaine et l'Armement voient les courses propres estimées ; les autres postes ne reçoivent pas ces mesures.
- Seul le Capitaine reçoit l'observation visuelle de la gerbe. L'Armement peut entendre une explosion sans recevoir sa position visuelle.
- Tester à 360 px et sur téléphone réel : carte, défilement du suivi, bouton sonore de 44 px minimum et périscope sans interception des gestes par les gerbes.
- Vérifier le son sur les navigateurs cibles après une interaction utilisateur, et le maintien de toutes les informations utiles lorsque l'audio est bloqué ou coupé.

## Vérifications automatisées

- `make check-all` : compilation native des crates de domaine/serveur, client WASM.
- `make test` : protocole, simulation et serveur, notamment impact hors écoute vu/non vu, limites du périscope et filtrage du nouvel événement.
- `cargo check -p client --tests --target wasm32-unknown-unknown` : compilation des tests client (géométrie de carte, effets sans rejeu, source audio synthétique, historique). Cette commande ne les exécute pas.

La météo, l'occultation entre bâtiments, les effets de naufrage détaillés et le bilan M6 ne font pas partie de cet incrément.
