SELECT row_to_json(action_prototypes.*) AS object
FROM action_prototypes_v1($1, $2) AS action_prototypes 
WHERE
    action_prototypes.schema_variant_id = $3
ORDER BY
    schema_variant_id DESC;
