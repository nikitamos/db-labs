CREATE FUNCTION op_to_act(op text) RETURNS AuditAction AS $$
BEGIN
RAISE LOG 'triggered on `%`', op;
IF op = 'DELETE' THEN RETURN 'delete'::AuditAction;
ELSIF op = 'INSERT' THEN RETURN 'create'::AuditAction;
ELSE RETURN 'update'::AuditAction;
END IF;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION audit_logger() RETURNS trigger AS $$
DECLARE
p_entity_id integer;
p_description JSONB;
BEGIN
IF TG_OP = 'DELETE' THEN
    p_entity_id = OLD.id;
    p_description = to_jsonb(OLD);
ELSE
    p_entity_id = NEW.id;
    p_description = to_jsonb(NEW);
END IF;

INSERT INTO audit_log (
    event_timestamp,
    user_id,
    table_name,
    entity_id,
    action,
    description
) VALUES (
    NOW(),
    0,
    TG_TABLE_NAME,
    p_entity_id,
    op_to_act(TG_OP),
    p_description
);
RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION audit_session_logger() RETURNS trigger AS $$
DECLARE
p_entity_id integer;
p_description JSONB;
BEGIN
IF TG_OP = 'DELETE' THEN
    p_entity_id = OLD.id;
    p_description = jsonb_build_object('user_id', OLD.user_id);
ELSE
    p_entity_id = NEW.id;
    p_description = jsonb_build_object('user_id', NEW.user_id);
END IF;

INSERT INTO audit_log (
    event_timestamp,
    user_id,
    table_name,
    entity_id,
    action,
    description
) VALUES (
    NOW(),
    0,
    TG_TABLE_NAME,
    p_entity_id,
    op_to_act(TG_OP),
    p_description
);
RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION audit_m2m_logger() RETURNS trigger AS $$
DECLARE
p_description JSONB;
BEGIN
IF TG_OP = 'DELETE' THEN
    p_description = to_jsonb(OLD);
ELSE
    p_description = to_jsonb(NEW);
END IF;

INSERT INTO audit_log (
    event_timestamp,
    user_id,
    table_name,
    entity_id,
    action,
    description
) VALUES (
    NOW(),
    0,
    TG_TABLE_NAME,
    0,
    op_to_act(TG_OP),
    p_description
);
RETURN NULL;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER trg_audit_logger_session
AFTER INSERT OR UPDATE OR DELETE ON sessions
FOR EACH ROW EXECUTE FUNCTION audit_session_logger();

CREATE TRIGGER trg_audit_logger_user
AFTER INSERT OR UPDATE OR DELETE ON users
FOR EACH ROW EXECUTE FUNCTION audit_logger();

CREATE TRIGGER trg_audit_logger_role
AFTER INSERT OR UPDATE OR DELETE ON roles
FOR EACH ROW EXECUTE FUNCTION audit_logger();

CREATE TRIGGER trg_audit_logger_permissions
AFTER INSERT OR UPDATE OR DELETE ON permissions
FOR EACH ROW EXECUTE FUNCTION audit_logger();

CREATE TRIGGER trg_audit_logger_projects
AFTER INSERT OR UPDATE OR DELETE ON projects
FOR EACH ROW EXECUTE FUNCTION audit_logger();

CREATE TRIGGER trg_audit_logger_categories
AFTER INSERT OR UPDATE OR DELETE ON categories
FOR EACH ROW EXECUTE FUNCTION audit_logger();

CREATE TRIGGER trg_audit_logger_statuses
AFTER INSERT OR UPDATE OR DELETE ON statuses
FOR EACH ROW EXECUTE FUNCTION audit_logger();

CREATE TRIGGER trg_audit_logger_tasks
AFTER INSERT OR UPDATE OR DELETE ON tasks
FOR EACH ROW EXECUTE FUNCTION audit_logger();

CREATE TRIGGER trg_audit_logger_discussions
AFTER INSERT OR UPDATE OR DELETE ON discussion_posts
FOR EACH ROW EXECUTE FUNCTION audit_logger();

CREATE TRIGGER trg_audit_logger_role_permissions
AFTER INSERT OR UPDATE OR DELETE ON role_permissions
FOR EACH ROW EXECUTE FUNCTION audit_m2m_logger();

CREATE TRIGGER trg_audit_logger_user_roles
AFTER INSERT OR UPDATE OR DELETE ON user_roles
FOR EACH ROW EXECUTE FUNCTION audit_m2m_logger();

CREATE TRIGGER trg_audit_logger_project_members
AFTER INSERT OR UPDATE OR DELETE ON project_members
FOR EACH ROW EXECUTE FUNCTION audit_m2m_logger();

CREATE TRIGGER trg_audit_logger_tasks_dependencies
AFTER INSERT OR UPDATE OR DELETE ON tasks_dependencies
FOR EACH ROW EXECUTE FUNCTION audit_m2m_logger();

CREATE TRIGGER trg_audit_logger_task_categories
AFTER INSERT OR UPDATE OR DELETE ON task_categories
FOR EACH ROW EXECUTE FUNCTION audit_m2m_logger();
