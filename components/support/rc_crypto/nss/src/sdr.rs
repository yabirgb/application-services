/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{
    error::*,
    pk11::types::ScopedSECItem,
    util::{ensure_nss_initialized, map_nss_secstatus, sec_item_as_slice},
};
use std::{
    convert::TryFrom,
    os::raw::{c_uchar, c_uint},
    ptr,
};

#[derive(Debug, Copy, Clone, PartialEq)]
enum Operation {
    Encrypt,
    Decrypt,
}

/// This implements NSS's secret decoder ring decryption, as described on
/// https://searchfox.org/mozilla-central/rev/3366c3d24f1c3818df37ec0818833bf085e41a53/security/manager/ssl/SecretDecoderRing.cpp#96-125.
/// Note that it only works on databases with no master password set.
pub fn encrypt(plaintext: &[u8]) -> Result<Vec<u8>> {
    common_crypt(plaintext, Operation::Encrypt)
}

/// This implements NSS's secret decoder ring decryption, as described on
/// https://searchfox.org/mozilla-central/rev/3366c3d24f1c3818df37ec0818833bf085e41a53/security/manager/ssl/SecretDecoderRing.cpp#131-151.
/// Note that it only works on databases with no master password set.
pub fn decrypt(ciphertext: &[u8]) -> Result<Vec<u8>> {
    common_crypt(ciphertext, Operation::Decrypt)
}

fn common_crypt(data: &[u8], operation: Operation) -> Result<Vec<u8>> {
    ensure_nss_initialized();
    let mut key_id = nss_sys::SECItem {
        type_: nss_sys::SECItemType::siBuffer,
        data: ptr::null_mut(),
        len: 0,
    };
    let mut request = nss_sys::SECItem {
        type_: nss_sys::SECItemType::siBuffer,
        data: data.as_ptr() as *mut c_uchar,
        len: c_uint::try_from(data.len())?,
    };
    let mut reply = ScopedSECItem::empty(nss_sys::SECItemType::siBuffer);
    map_nss_secstatus(|| unsafe {
        match operation {
            Operation::Decrypt => {
                nss_sys::PK11SDR_Decrypt(&mut request, reply.as_mut_ref(), std::ptr::null_mut())
            }
            Operation::Encrypt => nss_sys::PK11SDR_Encrypt(
                &mut key_id,
                &mut request,
                reply.as_mut_ref(),
                std::ptr::null_mut(),
            ),
        }
    })?;
    let output = unsafe { sec_item_as_slice(&mut reply)?.to_vec() };
    Ok(output)
}
