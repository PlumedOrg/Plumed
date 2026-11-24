// Regenerates the derived crypto values of test-vectors.json after a protocol
// version bump (the HKDF salt contains the version). The pairing secret is
// fixed to 0x03 * 32:
//   node protocol/gen-test-vectors.mjs protocol/test-vectors.json <version>

import {readFileSync,writeFileSync} from 'fs'; import c from 'crypto';
const p=process.argv[2], ver=process.argv[3]; const v=JSON.parse(readFileSync(p)); const k=v.crypto;
const d=Buffer.from(k.desktop_public,'base64'), ph=Buffer.from(k.phone_public,'base64'), x=Buffer.concat([Buffer.from(k.shared_x,'hex'),Buffer.alloc(32,3)]); k.pairing_secret=Buffer.alloc(32,3).toString('hex');
const key=l=>Buffer.from(c.hkdfSync('sha256',x,Buffer.from('Plumed-RemoteMic-'+ver),Buffer.concat([Buffer.from(l),d,ph]),32));
const p2d=key('phone-to-desktop'), d2p=key('desktop-to-phone');
const iv=Buffer.alloc(12,0x0a); const ci=c.createCipheriv('aes-256-gcm',p2d,iv);
const ct=Buffer.concat([ci.update(Buffer.from(k.packet_inner,'hex')),ci.final(),ci.getAuthTag()]);
k.key_phone_to_desktop=p2d.toString('hex'); k.key_desktop_to_phone=d2p.toString('hex'); k.hello_proof=key('hello-proof').toString('hex'); k.packet=Buffer.concat([iv,ct]).toString('hex');
const out={_comment:v._comment, protocol_version:ver, ...Object.fromEntries(Object.entries(v).filter(([n])=>n!=='_comment'&&n!=='protocol_version'))};
writeFileSync(p, JSON.stringify(out,null,2)+'\n');
