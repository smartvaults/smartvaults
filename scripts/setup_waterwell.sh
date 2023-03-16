export LEE_KEY=3fec18a9e196fd3a6417b45fad7005edb23d8529cb41d8ac738cfdd7d2b75677
export SARAH_KEY=7a38e12cbf7fa007d9c09d853f8aba0542aa4a9c7572d7497a9e7abb325b2af9
export JOHN_KEY=d599da2113d1c5ac7ad24a190884cd19a90c5fc45d90d446273647b2224f30f2
export MARIA_KEY=c4e5a7e19e371dc1296e95cb307b235aac5317aebffde18172d357507f2dd65f
export RACHEL_KEY=8676c56542ad4b43b1d6adb03ac00d00c41659236576455db12f48eb1657cbc3
export JAMES_KEY=a879bc870998dcacabcc4cfaa7e76387f55c8a5d307222de4db8df182b20e333
export KAREN_KEY=2c9d61eff72a9f5952a37af544db733f96082c2cfc06301f86567ccf76ae8a60
export MARK_KEY=5e6c9d26c38792e5120ddd29e288b2000ebd1d1dc58930e106ec7d0f66ada045
export AMANDA_KEY=eb65c8842c7443b9f660366d03834e604cf001043114719b1d12069a8f8f06b9

~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $LEE_KEY update-metadata --name "Lee" -a "Treasurer, Water Well" -p "https://marketplace.canva.com/EAFEits4-uw/1/0/1600w/canva-boy-cartoon-gamer-animated-twitch-profile-photo-oEqs2yqaL8s.jpg" --nip05 "lee@waterwell.ngo"
~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $SARAH_KEY update-metadata -n "sarah" -a "Chairperson, Water Well" -p "https://writestylesonline.com/wp-content/uploads/2019/01/What-To-Wear-For-Your-Professional-Profile-Picture-or-Headshot.jpg" --nip05 "sarah@waterwell.ngo"
~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $JOHN_KEY update-metadata -n "john" -a "Vice-Chairperson, Water Well" -p "https://img.freepik.com/premium-vector/profile-icon-male-avatar-hipster-man-wear-headphones_48369-8728.jpg" --nip05 "john@waterwell.ngo"
~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $MARIA_KEY update-metadata -n "maria" -a "Secretary, Water Well" -p "https://pixlr.com/studio/template/6264364c-b8cc-4f4f-92d8-28c69a2b756w/thumbnail.webp" --nip05 "maria@waterwell.ngo"
~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $RACHEL_KEY update-metadata -n "rachel" -a "Program Director, Water Well" -p "https://waterwell.ngo/profiles/rachel.png" --nip05 "rachel@waterwell.ngo"
~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $JAMES_KEY update-metadata -n "james" -a "Outreach Director, Water Well" -p "https://waterwell.ngo/profiles/james.png" --nip05 "james@waterwell.ngo"
~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $KAREN_KEY update-metadata -n "karen" -a "Board Member, Water Well" -p "https://waterwell.ngo/profiles/karen.png" --nip05 "karen@waterwell.ngo"
~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $MARK_KEY update-metadata -n "mark" -a "Board Member, Water Well" -p "https://waterwell.ngo/profiles/mark.png" --nip05 "mark@waterwell.ngo"
~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $AMANDA_KEY update-metadata -n "amanda" -a "Board Member, Water Well" -p "https://waterwell.ngo/profiles/amanda.png" --nip05 "amanda@waterwell.ngo"

~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $LEE_KEY publish-contact-list-csv -f contact-list.csv
~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $SARAH_KEY publish-contact-list-csv -f contact-list.csv
~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $JOHN_KEY publish-contact-list-csv -f contact-list.csv
~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $MARIA_KEY publish-contact-list-csv -f contact-list.csv
~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $RACHEL_KEY publish-contact-list-csv -f contact-list.csv
~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $JAMES_KEY publish-contact-list-csv -f contact-list.csv
~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $KAREN_KEY publish-contact-list-csv -f contact-list.csv
~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $MARK_KEY publish-contact-list-csv -f contact-list.csv
~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $AMANDA_KEY publish-contact-list-csv -f contact-list.csv

export TREY_KEY=4553e56832e5b6c4b83052f3b1d02d3d7b159ec5245c3cd97eda06683463744b
~/github.com/0xtrr/nostr-tool/target/release/nostr-tool -r wss://relay.rip --private-key $TREY_KEY update-metadata -n "trey" -a "Husband, Father, Son, and Board Member, Water Well" -p "https://waterwell.ngo/profiles/trey.png" 
