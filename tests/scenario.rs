use amcl_wrapper::field_elem::{FieldElement, FieldElementVector};
use amcl_wrapper::group_elem::{GroupElement, GroupElementVector};
use ps_sig::keys::{keygen, Params};
use ps_sig::blind_signature::*;
use ps_sig::pok_sig::*;
use ps_sig::{VerkeyGroupVec, SignatureGroup};
use std::collections::{HashMap, HashSet};

#[test]
fn test_scenario_1() {
    // User request signer to sign 10 messages where signer knows only 8 messages, the rest 2 are given in a form of commitment.
    // Once user gets the signature, it engages in a proof of knowledge of signature with a verifier.
    // The user also reveals to the verifier some of the messages.
    let count_msgs = 10;
    let count_blinded_msgs = 2;
    let params = Params::new("test".as_bytes());
    let (sk, vk) = keygen(count_msgs, &params);

    let blinding_key = BlindingKey::new(&sk, &params);
    let msgs = (0..count_msgs).map(|_| FieldElement::random()).collect::<Vec<FieldElement>>();
    let blinding = FieldElement::random();

    // User commits to some messages
    let mut comm = SignatureGroup::new();
    for i in 0..count_blinded_msgs {
        comm += (&blinding_key.Y[i] * &msgs[i]);
    }
    comm += (&params.g * &blinding);

    {
        // User and signer engage in a proof of knowledge for the above commitment `comm`
        let mut bases = Vec::<SignatureGroup>::new();
        let mut hidden_msgs = Vec::<FieldElement>::new();
        for i in 0..count_blinded_msgs {
            bases.push(blinding_key.Y[i].clone());
            hidden_msgs.push(msgs[i].clone());
        }
        bases.push(params.g.clone());
        hidden_msgs.push(blinding.clone());

        // User creates a random commitment, computes challenge and response. The proof of knowledge consists of commitment and responses
        let mut committing = ProverCommittingSignatureGroup::new();
        for b in &bases {
            committing.commit(b, None);
        }
        let committed = committing.finish();

        // Note: The challenge may come from the main protocol
        let chal = committed.gen_challenge(comm.to_bytes());

        let proof = committed.gen_proof(&chal, hidden_msgs.as_slice()).unwrap();

        // Signer verifies the proof of knowledge.
        assert!(proof.verify(bases.as_slice(), &comm, &chal).unwrap());
    }

    // Get signature, unblind it and then verify.
    let sig_blinded = BlindSignature::new(
        &comm,
        &msgs.as_slice()[count_blinded_msgs..count_msgs],
        &sk,
        &blinding_key,
        &params
    )
    .unwrap();
    let sig_unblinded = BlindSignature::unblind(&sig_blinded, &blinding);
    assert!(sig_unblinded.verify(msgs.clone(), &vk, &params).unwrap());

    // Do a proof of knowledge of the signature and also reveal some of the messages.
    let mut revealed_msg_indices = HashSet::new();
    revealed_msg_indices.insert(4);
    revealed_msg_indices.insert(6);
    revealed_msg_indices.insert(9);

    let pok = PoKOfSignature::init(
        &sig_unblinded,
        &vk, &params,
        msgs.clone(),
        None,
        revealed_msg_indices.clone(),
    )
    .unwrap();

    let chal = pok.pok_vc.gen_challenge(pok.J.to_bytes());

    let proof = pok.gen_proof(&chal).unwrap();

    let mut revealed_msgs = HashMap::new();
    for i in &revealed_msg_indices {
        revealed_msgs.insert(i.clone(), msgs[*i].clone());
    }
    assert!(proof.verify(&vk, &params, revealed_msgs.clone(), &chal).unwrap());
}
