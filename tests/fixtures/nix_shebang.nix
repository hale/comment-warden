{ pkgs, ... }:
{
  systemd.services.op-auth = {
    serviceConfig = {
      # This file contains the service account token, generated and pushed to the node by deploy.nix
      EnvironmentFile = "/etc/onepassword/service-account";
      ExecStart = pkgs.writeScript "op-auth-start" ''
        #!${pkgs.bash}/bin/bash
        export OP_SERVICE_ACCOUNT_TOKEN=$OP_SERVICE_ACCOUNT_TOKEN
        # Verify authentication by listing vaults
        ${pkgs._1password-cli}/bin/op vault list > /dev/null
        echo "1Password CLI authenticated successfully"
      '';
      # TRIPWIRE: removing this restart policy leaves the unit dead after a token rotation
      Restart = "on-failure";
    };
  };
}
