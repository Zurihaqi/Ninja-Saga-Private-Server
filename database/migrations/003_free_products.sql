-- Local free-product claims replace payment processing. No payment data is stored.
CREATE TABLE IF NOT EXISTS free_product_claims (
  id bigint unsigned NOT NULL AUTO_INCREMENT PRIMARY KEY,
  user_id bigint unsigned NOT NULL,
  character_id bigint unsigned NOT NULL,
  request_id varchar(128) NOT NULL,
  product_id varchar(128) NOT NULL,
  created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE KEY free_product_request (user_id, request_id),
  CONSTRAINT free_product_user_fk FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
  CONSTRAINT free_product_character_fk FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
