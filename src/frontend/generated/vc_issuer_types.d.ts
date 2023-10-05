import type { Principal } from '@dfinity/principal';
export interface CredentialSpec { 'info' : string }
export interface GetCredentialRequest {
  'signed_id_alias' : SignedIdAlias,
  'prepared_context' : [] | [Array<number>],
  'credential_spec' : CredentialSpec,
}
export type GetCredentialResponse = { 'Ok' : IssuedCredentialData } |
  { 'Err' : IssueCredentialError };
export interface Icrc21ConsentInfo {
  'consent_message' : string,
  'language' : string,
}
export interface Icrc21ConsentMessageRequest {
  'arg' : Array<number>,
  'method' : string,
  'preferences' : Icrc21ConsentPreferences,
}
export type Icrc21ConsentMessageResponse = { 'Ok' : Icrc21ConsentInfo } |
  { 'Err' : Icrc21Error };
export interface Icrc21ConsentPreferences { 'language' : string }
export type Icrc21Error = { 'GenericError' : Icrc21ErrorInfo } |
  { 'MalformedCall' : Icrc21ErrorInfo } |
  { 'NotSupported' : Icrc21ErrorInfo } |
  { 'Forbidden' : Icrc21ErrorInfo };
export interface Icrc21ErrorInfo {
  'description' : string,
  'error_code' : bigint,
}
export type IssueCredentialError = { 'Internal' : string } |
  { 'SignatureNotFound' : string } |
  { 'InvalidIdAlias' : string } |
  { 'UnauthorizedSubject' : string } |
  { 'UnknownSubject' : string };
export interface IssuedCredentialData { 'vc_jws' : string }
export interface PrepareCredentialRequest {
  'signed_id_alias' : SignedIdAlias,
  'credential_spec' : CredentialSpec,
}
export type PrepareCredentialResponse = { 'Ok' : PreparedCredentialData } |
  { 'Err' : IssueCredentialError };
export interface PreparedCredentialData {
  'prepared_context' : [] | [Array<number>],
}
export interface SignedIdAlias {
  'credential_jws' : string,
  'id_alias' : Principal,
  'id_dapp' : Principal,
}
export interface _SERVICE {
  'consent_message' : (arg_0: Icrc21ConsentMessageRequest) => Promise<
      Icrc21ConsentMessageResponse
    >,
  'get_credential' : (arg_0: GetCredentialRequest) => Promise<
      GetCredentialResponse
    >,
  'prepare_credential' : (arg_0: PrepareCredentialRequest) => Promise<
      PrepareCredentialResponse
    >,
}
