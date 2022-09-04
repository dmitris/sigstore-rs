//
// Copyright 2021 The Sigstore Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! This crate aims to provide [Sigstore](https://www.sigstore.dev/) capabilities to Rust developers.
//!
//! Currently, the main focus of the crate is to provide the verification
//! capabilities offered by the official `cosign` tool.
//!
//! **Warning:** this library is still experimental. Its API can change at any time.
//!
//! # Security
//!
//! Should you discover any security issues, please refer to
//! Sigstore's [security process](https://github.com/sigstore/community/blob/main/SECURITY.md).
//!
//! # Verification
//!
//! Sigstore verification is done using the [`cosign::Client`](crate::cosign::Client)
//! struct.
//!
//! ## Triangulation of Sigstore signature
//!
//! Given a container image/oci artifact, calculate the location of
//! its cosign signature inside of a registry:
//!
//! ```rust,no_run
//! use crate::sigstore::cosign::CosignCapabilities;
//! use std::fs;
//!
//! #[tokio::main]
//! pub async fn main() {
//!   let auth = &sigstore::registry::Auth::Anonymous;
//!
//!   let mut client = sigstore::cosign::ClientBuilder::default()
//!     .build()
//!     .expect("Unexpected failure while building Client");
//!   let image = "registry-testing.svc.lan/kubewarden/disallow-service-nodeport:v0.1.0";
//!   let (cosign_signature_image, source_image_digest) = client.triangulate(
//!     image,
//!     auth
//!   ).await.expect("Unexpected failure while using triangulate");
//! }
//! ```
//!
//! ## Signature verification
//!
//! Verify the signature of a container image/oci artifact:
//!
//! ```rust,no_run
//! use crate::sigstore::cosign::{
//!     CosignCapabilities,
//!     verify_constraints,
//! };
//! use crate::sigstore::cosign::verification_constraint::{
//!     AnnotationVerifier,
//!     PublicKeyVerifier,
//!     VerificationConstraintVec,
//! };
//! use crate::sigstore::crypto::SignatureDigestAlgorithm;
//! use crate::sigstore::errors::SigstoreError;
//! use sigstore::errors::SigstoreVerifyConstraintsError;
//!
//! use std::boxed::Box;
//! use std::collections::HashMap;
//! use std::fs;
//!
//! #[tokio::main]
//! pub async fn main() {
//!   let auth = &sigstore::registry::Auth::Anonymous;
//!
//!   // Provide both rekor and fulcio data -> this enables keyless verification
//!   // Read rekor's key from the location generated by `cosign initialize`
//!   let rekor_pub_key = fs::read_to_string("~/.sigstore/root/targets/rekor.pub")
//!     .expect("Cannot read rekor public key");
//!   // Read fulcio's certificate from the location generated by `cosign initialize`
//!   let fulcio_cert = fs::read_to_string("~/.sigstore/root/targets/fulcio.crt.pem")
//!     .expect("Cannot read fulcio certificate");
//!
//!   let mut client = sigstore::cosign::ClientBuilder::default()
//!     .with_rekor_pub_key(&rekor_pub_key)
//!     .with_fulcio_cert(fulcio_cert.as_bytes())
//!     .build()
//!     .expect("Unexpected failure while building Client");
//!
//!   // Obtained via `triangulate`
//!   let cosign_image = "registry-testing.svc.lan/kubewarden/disallow-service-nodeport:sha256-5f481572d088dc4023afb35fced9530ced3d9b03bf7299c6f492163cb9f0452e.sig";
//!   // Obtained via `triangulate`
//!   let source_image_digest = "sha256-5f481572d088dc4023afb35fced9530ced3d9b03bf7299c6f492163cb9f0452e";
//!
//!   // Obtain the list the signatures layers associated that can be trusted
//!   let signature_layers = client.trusted_signature_layers(
//!     auth,
//!     cosign_image,
//!     source_image_digest,
//!   ).await.expect("Could not obtain signature layers");
//!
//!   // Define verification constraints
//!   let mut annotations: HashMap<String, String> = HashMap::new();
//!   annotations.insert("env".to_string(), "prod".to_string());
//!   let annotation_verifier = AnnotationVerifier{
//!     annotations,
//!   };
//!
//!   let verification_key = fs::read("~/cosign.pub")
//!     .expect("Cannot read contents of cosign public key");
//!   let pub_key_verifier = PublicKeyVerifier::new(
//!     &verification_key,
//!     SignatureDigestAlgorithm::default(),
//!   ).expect("Could not create verifier");
//!
//!   let verification_constraints: VerificationConstraintVec = vec![
//!     Box::new(annotation_verifier),
//!     Box::new(pub_key_verifier),
//!   ];
//!
//!   // Use the given list of constraints to verify the trusted
//!   // `signature_layers`. This will raise an error if one or more verification
//!   // constraints are not satisfied.
//!   let result = verify_constraints(
//!     &signature_layers,
//!     verification_constraints.iter());
//!
//!   match result {
//!       Ok(()) => {
//!           println!("Image successfully verified");
//!       }
//!       Err(SigstoreVerifyConstraintsError {
//!           unsatisfied_constraints,
//!       }) => {
//!           println!("{:?}", unsatisfied_constraints);
//!           panic!("Image verification failed")
//!       }
//!   }
//! }
//! ```
//! # Rekor integration
//! The examples folder contains code that shows users how to make Rekor API calls.  
//! It also provides a clean interface with step-by-step instructions that other developers can copy and paste.
//!
//! ```
//! use clap::{Arg, Command};
//! use sigstore::rekor::apis::{configuration::Configuration, entries_api};
//! use sigstore::rekor::models::log_entry::LogEntry;
//! use std::str::FromStr;
//! #[tokio::main]
//! async fn main() {
//!     /*
//!     Retrieves an entry and inclusion proof from the transparency log (if it exists) by index
//!     Example command :
//!     cargo run --example get_log_entry_by_index -- --log_index 99
//!     */
//!     let matches = Command::new("cmd").arg(
//!         Arg::new("log_index")
//!             .long("log_index")
//!             .takes_value(true)
//!             .help("log_index of the artifact"),
//!     );
//!
//!     let flags = matches.get_matches();
//!     let index = <i32 as FromStr>::from_str(flags.value_of("log_index").unwrap_or("1")).unwrap();
//!
//!     let configuration = Configuration::default();
//!
//!     let message: LogEntry = entries_api::get_log_entry_by_index(&configuration, index)
//!         .await
//!         .unwrap();
//!     println!("{:#?}", message);
//! }
//! ```
//!
//! The following comment in the code tells the user how to provide the required values to the API calls using cli flags.
//!
//! In the example below, the user can retrieve different entries by inputting a different value for the log_index flag.
//!
//!
//!/*
//!Retrieves an entry and inclusion proof from the transparency log (if it exists) by index
//!Example command :
//!cargo run --example get_log_entry_by_index -- --log_index 99
//!*/
//!
//! # The example code is provided for the following API calls:
//!
//!- create_log_entry
//!- get_log_entry_by_index
//!- get_log_entry_by_uuid
//!- get_log_info
//!- get_log_proof
//!- get_public_key
//!- get_timestamp_cert_chain
//!- get_timestamp_response
//!- search_index
//!- search_log_query
//!
//!
//! # Examples
//!
//! Additional examples can be found inside of the [`examples`](https://github.com/sigstore/sigstore-rs/tree/main/examples/)
//! directory.
//!
//! ## Fulcio and Rekor integration
//!
//! [`cosign::Client`](crate::cosign::Client) integration with Fulcio and Rekor
//! requires the following data to work: Fulcio's certificate and Rekor's public key.
//!
//! These files are safely distributed by the Sigstore project via a TUF repository.
//! The [`sigstore::tuf`](crate::tuf) module provides the helper structures to deal
//! with it.
//!

#![forbid(unsafe_code)]
#![warn(clippy::unwrap_used, clippy::panic)]

pub mod crypto;
mod mock_client;

pub mod cosign;
pub mod errors;
pub mod oauth;
pub mod registry;
pub mod rekor;
pub mod simple_signing;
pub mod tuf;
