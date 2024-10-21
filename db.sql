CREATE TABLE grsync_avatar
(
    eid VARCHAR(32) NOT NULL,  -- Email ID for MD5(email)
    rating tinyint NOT NULL DEFAULT 0, -- Image rating
    resource VARCHAR(40), -- Resource SHA1 Unique ID

    create_time bigint NOT NULL DEFAULT 0, -- First request time
    last_update bigint NOT NULL DEFAULT 0,  -- Last Sync to the gr source server
    PRIMARY KEY (eid, rating)
);

CREATE TABLE grsync_resource
(
    sha1 VARCHAR(40) NOT NULL PRIMARY KEY, -- Image SHA1
    resource VARCHAR(255), -- Resource on path || if null , path = "/${SYS_CONF_PATH}/${SHA1}.avif"

    size int NOT NULL DEFAULT 512,-- Image size

    origin_url VARCHAR(255) -- Origin pull url
);


CREATE VIEW grsync_query AS
SELECT eid, rating, gr.sha1, gr.resource,size,last_update
FROM grsync_avatar ga
LEFT OUTER JOIN grsync_resource gr on ga.resource = gr.sha1;