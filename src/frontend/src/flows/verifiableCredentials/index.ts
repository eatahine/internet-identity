import { SignedIdAlias } from "$generated/internet_identity_types";
import { authenticateBox } from "$src/components/authenticateBox";
import { showSpinner } from "$src/components/spinner";
import { toast } from "$src/components/toast";
import { getDapps } from "$src/flows/dappsExplorer/dapps";
import { authnTemplateManage } from "$src/flows/manage";
import { I18n } from "$src/i18n";
import { AuthenticatedConnection, Connection } from "$src/utils/iiConnection";
import { Principal } from "@dfinity/principal";
import { allow } from "./allow";
import { vcProtocol } from "./postMessageInterface";

const dapps = getDapps();

const giveUp = async (message?: string): Promise<never> => {
  console.error("Nope " + message);
  toast.error("Nope " + message);
  return await new Promise((_) => {
    /* halt */
  });
};

export const vcFlow = async ({ connection }: { connection: Connection }) => {
  const _result = await vcProtocol({
    onProgress: (x) => {
      if (x === "waiting") {
        return showSpinner({
          message: "Waiting for info",
        });
      }

      if (x === "verifying") {
        return showSpinner({
          message: "Verifying",
        });
      }
      x satisfies never;
    },
    verifyCredentials: async ({ request, rpOrigin }) => {
      // Go through the login flow, potentially creating an anchor.
      const { connection: authenticatedConnection } = await authenticateBox({
        connection,
        i18n: new I18n(),
        templates: authnTemplateManage({ dapps }),
      });

      const { issuerOrigin } = request.params.issuer;

      const { hostname: rpHostname } = new URL(rpOrigin);
      const computedP_RP = await authenticatedConnection.getPrincipal({
        hostname: rpHostname,
      });

      const pAliasPending = getAlias({
        rpOrigin,
        issuerOrigin,
        authenticatedConnection,
      });

      const givenP_RP = Principal.fromText(request.params.credentialSubject);
      // TODO: do some proper principal checking
      if (computedP_RP.toString() !== givenP_RP.toString()) {
        return giveUp(
          [
            "bad principals",
            computedP_RP.toString(),
            givenP_RP.toString(),
          ].join(", ")
        );
      }

      const allowed = await allow({
        relyingOrigin: rpOrigin,
        providerOrigin: issuerOrigin,
      });
      if (allowed === "canceled") {
        return giveUp("canceled");
      }
      allowed satisfies "allowed";

      const [_pAlias] = await Promise.all([pAliasPending]);

      return giveUp("Rest of flow not implemented");

      return {
        verifiablePresentation: "hello",
      };
    },
  });
};

const _lookupCanister = ({
  origin,
}: {
  origin: string;
}): Promise<Principal> => {
  return giveUp("Don't know how to lookup canister " + origin);
};

const getAlias = async ({
  authenticatedConnection,
  issuerOrigin,
  rpOrigin,
}: {
  issuerOrigin: string;
  rpOrigin: string;
  authenticatedConnection: AuthenticatedConnection;
}): Promise<{
  rpAliasCredential: SignedIdAlias;
  issuerAliasCredential: SignedIdAlias;
}> => {
  const preparedIdAlias = await authenticatedConnection.prepareIdAlias({
    issuerOrigin,
    rpOrigin,
  });

  if ("error" in preparedIdAlias) {
    return giveUp("Could not prepare alias");
  }

  const result = await authenticatedConnection.getIdAlias({
    preparedIdAlias,
    issuerOrigin,
    rpOrigin,
  });

  if ("error" in result) {
    return giveUp("Could not get alias");
  }

  const {
    rp_id_alias_credential: rpAliasCredential,
    issuer_id_alias_credential: issuerAliasCredential,
  } = result;

  return { rpAliasCredential, issuerAliasCredential };
};
