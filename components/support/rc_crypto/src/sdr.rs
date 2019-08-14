/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

pub use nss::sdr::{decrypt, encrypt};

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let data = b"When I grow up, I want to be a watermelon";
        let ciphertext = encrypt(data).unwrap();
        let plaintext = encrypt(&ciphertext).unwrap();
        assert_eq!(data.to_vec(), plaintext);
    }
}
