# Recette M5.2 — Riposte et dégâts

## Préparation

1. Lancer `make server` et `make client` dans deux terminaux.
2. Ouvrir `http://127.0.0.1:8080` dans quatre onglets : Pilote (création), Sonar, Capitaine et Ingénierie (même code de salle).
3. Passer les quatre joueurs prêts et démarrer. Garder le Capitaine humain, sans autoriser de tir : ne pas laisser un bot couler l'escorte pendant cette recette.
4. Recharger tous les onglets après une mise à jour : le protocole actuel est v9, incompatible v8/v7. Chaque scénario comparatif doit repartir d'une salle neuve ; ne pas réparer pendant cette recette de dégâts bruts.

## 1. État initial et approche

- Vérifier `COQUE 100% / EAU 0%` aux quatre postes.
- En Ingénierie, vérifier les trois compartiments à 100%, sans fuite ni historique de dégâts.
- Pilote : descendre à 100 m, prendre le cap nord à 8 nœuds pour se rapprocher du convoi initialement au nord. Utiliser les estimations du Sonar pour corriger l'approche, pas une position exacte ennemie.
- Sonar : émettre périodiquement, identifier la piste probable d'escorte et la partager. Le rapprochement peut prendre plusieurs minutes ; l'IA n'attaque pas automatiquement après un délai fixé.

Attendu : les pings ennemis deviennent perceptibles quand l'escorte recherche. Un ping entendu ne confirme ni écho ni largage. Aucun état interne de l'IA, munition ennemie ou position de charge ne doit apparaître.

## 2. Subir une passe

- Une fois l'escorte estimée proche, réduire à 0–2 nœuds, rester vers 100 m et continuer ponctuellement les pings pour provoquer la recherche.
- Attendre la riposte. L'attaque nécessite une estimation assez récente et précise, puis une approche à portée de largage ; une absence d'attaque ne constitue pas seule un échec.
- Surveiller le bandeau `EXPLOSION ENTENDUE` aux quatre postes ouverts, puis coque/eau et l'Ingénierie. Il est également disponible à l'Armement si un cinquième humain rejoint avant le démarrage.

Attendu en cas de coup proche : baisse de coque, intégrité d'au moins un compartiment réduite, fuite positive, puis hausse de l'eau au fil des ticks. Les alertes `COQUE ENDOMMAGEE` et `VOIE D'EAU` restent présentes dans les snapshots. L'Ingénierie conserve tick, cause et perte de coque ; les autres postes ne reçoivent pas le détail des compartiments. Une explosion audible peut manquer et ne provoquer aucun dégât.

Vérifier aussi le bandeau : il disparaît après 3 secondes sans nouvelle explosion ; chaque nouvelle explosion relance sa durée, et les explosions rapprochées affichent un compteur sans empilement. Les explosions inaudibles et les simples pings ne le déclenchent pas. Il ne révèle ni direction, ni distance, ni résultat d'impact. Les commandes restent visibles et cliquables/tactiles pendant l'alerte, y compris en 360×640 et 640×360. Les impacts de torpille audibles doivent produire la même alerte pour tout l'équipage, sans double comptage dans les postes qui reçoivent aussi l'événement d'impact identifié.

## 3. Persistance et profondeur

- Après un coup, cesser les pings, s'éloigner et observer pendant une à deux minutes.
- Vérifier que la fuite et l'inondation ne disparaissent pas avec le message d'explosion.
- Comparer consigne de profondeur et valeur réelle : la tenue se dégrade ; une forte inondation peut dépasser la capacité de remontée. Tester la remontée d'urgence si nécessaire.
- Ne pas activer de pompage, isolation ou réparation pendant ce contrôle de persistance. Pour ces commandes désormais livrées, poursuivre avec [la recette M5.3](recette-m5.3.md).

## 4. Esquive, dans une nouvelle salle

- Refaire l'approche, puis changer franchement de route et de profondeur à proximité de l'escorte tout en maintenant de la vitesse.
- Comparer les dégâts avec le scénario immobile. Les charges déjà larguées n'adaptent ni leurs coordonnées ni leur profondeur à la manœuvre.

La date exacte de largage étant volontairement cachée, ce test manuel est indicatif, pas une preuve d'absence de guidage. Les tests déterministes couvrent directement la trajectoire après largage et l'indépendance des décisions face aux mouvements non observés.

## 5. Destruction et interfaces

- Dans une salle dédiée, rester sous les attaques ou demander 250 m et attendre la perte de coque par pression.
- Attendu : bandeau rouge permanent `SOUS-MARIN PERDU` aux cinq postes, cause conservée, déplacement immobilisé, nouvelles commandes refusées explicitement. Le bandeau reste visible au-delà de 3 secondes, remplace l'alerte d'explosion et persiste après une déconnexion si le dernier état reçu confirme la perte. Pas encore d'écran de défaite ou bilan ; créer une nouvelle salle pour rejouer.
- Tester 1280×720, 360×640 et 640×360 : synthèse lisible, aucun chevauchement, zones Sonar indépendantes et Ingénierie défilante (molette et glissement tactile), commandes accessibles en défilant.

Pour chaque anomalie : noter rôle, scénario, tick ou temps écoulé, consignes, résultat et capture. L'équilibrage des dégâts et l'ergonomie sur téléphone réel restent à valider.
