# External boundaries

The image provider boundary accepts the compiled prompt plus typed output
parameters. PNG is the maintained artifact format and the backend does not
promise transparent or vector output.

Reference images are passed as explicit transported inputs with declared roles.
Descriptions alone never become hidden filesystem paths. Provider/network/auth
failures remain visible in the adapter result and do not turn into a successful
prompt or artifact claim.
