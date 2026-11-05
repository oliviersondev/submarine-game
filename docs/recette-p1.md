# Recette P1 — Périscope et contacts visuels

## Préparation

1. Lancer `make server` et `make client`, puis recharger les onglets : protocole v10 incompatible v9.
2. Créer une salle comme **Capitaine** (`http://127.0.0.1:8080/?role=captain`). En solo, les quatre autres postes sont des bots. Un démarrage avec un seul humain sur un autre poste doit être refusé sans lancer la simulation.
3. Pour vérifier le partage, faire rejoindre Sonar et/ou Armement depuis d'autres onglets avant le démarrage. Deux humains sans Capitaine restent autorisés, mais seul un Capitaine humain dispose des commandes de périscope.
4. Démarrer et ne pas autoriser de tir pendant cette recette. La détection acoustique existante reste active ; cette recette n'injecte pas d'ennemi ni d'état caché.

## Approche et observation

1. Au Capitaine, ouvrir **PERISCOPE / CARTE** dès le démarrage en surface et choisir **MONTER / BAISSER** : la montée doit réussir et permettre l'observation sans ordre de plongée préalable. Abaisser le périscope : vue noire, aucune silhouette.
2. Dans le panneau du périscope, régler les consignes du bot Pilote à **cap 000 / vitesse 2 kn / profondeur 12 m**, puis toucher **ORDONNER**. Si le poste Pilote est humain, **ORDONNER** est atténué et aucun ordre n'est envoyé ; lui demander la manœuvre. Le serveur conserve aussi son refus si un client lui envoie malgré tout un ordre au bot.
3. Choisir **MONTER / BAISSER** pendant la plongée : le périscope doit rester utilisable sur toute la plage réelle de 0 à 14 m inclus, à une vitesse au plus égale à 6 nœuds. Vérifier que l'arrivée à 12 m conserve cette disponibilité.
4. Observer le convoi initialement au nord. Tourner avec les boutons ±5° et par glissement horizontal : relèvement absolu, angle relatif à l'étrave, horizon et silhouettes. Le champ est de 40°, la portée de 8 km ; aucun zoom, nom ennemi ni distance exacte.
5. Vérifier que les silhouettes se déplacent et changent de taille apparente avec la géométrie ; leur profil dépend de l'orientation. Une silhouette passée hors champ disparaît, sans marqueur de sa position réelle sur la carte.

## Création, sélection et partage

1. Toucher une silhouette puis **CREER CONTACT**. Attendre le snapshot : un « Visuel N » est sélectionné avec description, relèvement, heure simulée et âge. Une nouvelle création sur cette même silhouette encore visible ne doit pas dupliquer le contact.
2. Créer un autre contact ; vérifier sa sélection après confirmation serveur. **CONTACT SUIVANT** permet de parcourir les contacts encore mémorisés.
3. **PARTAGER** le contact sélectionné. Sonar le retrouve dans le journal d'observations défilant ; Armement dans son panneau défilant. Avant partage, seuls le Capitaine et son écran périscopique connaissent ce contact. Pilote et Ingénierie n'en reçoivent pas les détails.
4. Le contact visuel n'apparaît pas parmi les pistes sélectionnables pour le tir, ne crée pas de solution de tir et n'est pas fusionné avec un contact sonar. Le partage doit fonctionner sans piste sonar préalable (couvert aussi par les tests serveur).

## Perte, ancienneté et expiration

1. Abaisser le périscope ou tourner suffisamment pour perdre le navire. Le dernier relèvement et son heure restent figés ; seul l'âge progresse, même si le navire bouge hors de vue.
2. Après 30 secondes simulées sans observation, le contact devient **ANCIEN**. Après 120 secondes, il disparaît de tous les destinataires.
3. Revoir entre-temps le même navire ne réassocie pas secrètement l'ancien contact. Sa silhouette peut faire l'objet d'une nouvelle création distincte. La mémoire est limitée à 64 contacts, sans croissance illimitée.

## Limites et interface

- Périscope monté, ordonner plus de 14 m ou plus de 6 nœuds : abaissement automatique, motif persistant et vue masquée. Revenir dans les limites ne remonte pas le périscope.
- Vérifier les limites inclusives de 0 m, 14 m et 6 nœuds, ainsi que l'utilisation entre la surface et la profondeur périscopique ; les tests de simulation contrôlent précisément ces valeurs. Une remontée vers la surface à vitesse autorisée ne doit pas abaisser le périscope.
- Tester 1280×720, 360×640 et 640×360, souris et tactile : boutons d'au moins 44 px, sélection, partage, défilement vertical du panneau et rotation horizontale de la vue. Un glissement ne doit pas activer un bouton.
- Sur les boutons du périscope, vérifier l'éclaircissement au survol souris, le fond/contour accentués et l'effet enfoncé pendant l'appui, puis un flash de 150 ms au clic ou au tap validé. Un tap rapide doit rester perceptible ; un appui prolongé ne répète pas la commande et un glissement ne déclenche pas de flash de clic. Ce retour confirme la prise en compte locale du geste, pas l'acceptation serveur. Sans silhouette sélectionnée/contact disponible, ou si l'action est déjà appliquée, les commandes concernées sont atténuées ; un ordre au bot Pilote est également atténué si celui-ci est humain. Un bouton MONTER indisponible peut toujours être touché pour expliquer le refus dans la zone noire, sans flash d'activation.
- Les erreurs doivent rester lisibles dans le panneau. Les bandeaux d'explosion et de perte gardent la priorité au-dessus de la vue.
- Périscope monté, vérifier les rubans gradués sur l'image : absolu en haut, relatif en bas (AV/TD/BD/AR), pas de 1°, grandes marques tous les 5° et chiffres tous les 10°. Tourner de part et d'autre de 000° : aucune rupture ni valeur 360°. Avec le sous-marin au cap 000 et la vue à 020°, les lectures centrales indiquent ABS 020 et REL TD 20. Faire tourner le sous-marin à 040° en gardant le périscope à 020° : l'absolu reste à 020 et le relatif devient BD 20. Les graduations ne doivent pas bloquer le glissement ni la sélection des silhouettes ; vérifier leur disparition à l'abaissement et leur lisibilité en 360 px.
- Périscope fermé, vérifier les messages au centre de la zone noire, et non dans le bandeau des instruments : « OUVERTURE IMPOSSIBLE » avec motif et limite, ou « PÉRISCOPE ABAISSÉ AUTOMATIQUEMENT ». Après retour dans les limites, l'invitation à monter remplace l'ancien motif ; après abaissement manuel, seul « PÉRISCOPE ABAISSÉ » reste affiché. Les messages disparaissent à la montée. Vérifier le retour à la ligne en portrait ; les erreurs sans rapport avec le périscope restent hors de la zone noire.

## État de validation

Les tests automatisés couvrent limites/visibilité, perte/ancienneté/expiration, absence de réassociation, bornage, déterminisme, fixtures `postcard`, permissions et projections par rôle, ainsi que le démarrage solo. Le parcours navigateur a été exercé avec rendu logiciel en 1280×720, puis avec émulation tactile en 360×640 (création, partage et rotation) et 640×360 (défilement du panneau), sans exception JavaScript observée. Cela ne valide ni les performances ni l'ergonomie sur téléphone réel. L'équilibrage reste provisoire.

## Report explicite — P2

**Le périscope n'est pas encore détectable visuellement par l'ennemi.** Ce report est temporaire et suivi dans la [roadmap P2](roadmap.md#p2--suivi-obligatoire--détection-du-périscope). Ne pas clôturer ce suivi lors de la validation de P1. Météo, vagues masquantes, occultation entre navires et détection aérienne ne sont pas simulées par P1.
