#:package Bogus@35.6.5
#:property PublishAot=false
#:property JsonSerializerIsReflectionEnabledByDefault=true

// Populates the DMaDBMS API using the endpoints described by the OpenAPI spec.
//
// Requirements: .NET 10+
// Run:
//   dotnet run populate.cs
//   dotnet run populate.cs -- --base-url http://localhost:44761
//   dotnet run populate.cs -- --count 60
//   dotnet run populate.cs -- --no-reset
//
// By default the script calls POST /db_reset first, then seeds users,
// sessions, categories, statuses, projects, project members, tasks, task
// categories, task dependencies and discussion posts through the HTTP API.
//
// Note: roles, permissions, role_permissions, user_roles and audit_log are not
// exposed by the HTTP API, so this script cannot seed those tables.

using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Bogus;

const string DefaultBaseUrl = "http://localhost:6200";
const int DefaultRecordCount = 60;
const string SeedPassword = "password123";

var baseUrl = DefaultBaseUrl;
var resetDatabase = true;
var recordCount = DefaultRecordCount;

for (var i = 0; i < args.Length; i++)
{
    switch (args[i])
    {
        case "--base-url" when i + 1 < args.Length:
            baseUrl = args[++i];
            break;
        case "--count" when i + 1 < args.Length:
            if (!int.TryParse(args[++i], out recordCount) || recordCount < 1)
            {
                Console.Error.WriteLine("--count must be a positive integer.");
                return 2;
            }
            break;
        case "--no-reset":
            resetDatabase = false;
            break;
        case "-h" or "--help":
            PrintUsage();
            return 0;
        default:
            Console.Error.WriteLine($"Unknown argument: {args[i]}");
            PrintUsage();
            return 2;
    }
}

var faker = new Faker();
using var api = new ApiClient(baseUrl);

try
{
    var health = await api.GetAsync<HealthResponse>("/health");
    Console.WriteLine($"API health at {baseUrl}: {health.Status}");
}
catch (Exception ex)
{
    Console.Error.WriteLine($"Cannot reach the API at {baseUrl}: {ex.Message}");
    return 1;
}

try
{
    if (resetDatabase)
    {
        Console.WriteLine("Resetting database...");
        await api.PostEmptyAsync("/db_reset");
    }
    else
    {
        Console.WriteLine("Skipping database reset (--no-reset).");
    }

    // ------------------------------------------------------------------
    // 1. Users. Signup hashes the password and returns the created user.
    // ------------------------------------------------------------------
    Console.WriteLine($"Creating {recordCount} users...");
    var users = new List<UserSeed>();
    var usedUsernames = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

    for (var i = 0; i < recordCount; i++)
    {
        var firstName = faker.Name.FirstName();
        var lastName = faker.Name.LastName();
        var username = CreateUsername(faker, firstName, i, usedUsernames);
        var email = $"{username}@example.com";

        var created = await api.PostAsync<EntityId>("/auth/signup", new SignupDto(
            Email: email,
            Username: username,
            Password: SeedPassword,
            FirstName: firstName,
            SecondName: lastName,
            Bio: faker.Lorem.Sentence(10)));

        users.Add(new UserSeed(created.RequireId(), username));
    }

    // ------------------------------------------------------------------
    // 2. Sessions. Logging in creates one session row per user.
    // ------------------------------------------------------------------
    Console.WriteLine($"Creating {recordCount} sessions...");
    foreach (var user in users)
    {
        await api.PostAsync<SessionResponse>("/auth/login", new SigninDto(
            Identity: user.Username,
            Password: SeedPassword));
    }

    // ------------------------------------------------------------------
    // 3. Categories.
    // ------------------------------------------------------------------
    Console.WriteLine($"Creating {recordCount} categories...");
    var categoryIds = new List<int>();
    for (var i = 0; i < recordCount; i++)
    {
        var created = await api.PostAsync<EntityId>("/category", new CreateCategoryDto(
            Name: Truncate(faker.Commerce.Categories(1)[0], 24),
            BadgeColor: faker.Random.Int(0, 0xFFFFFF),
            Urgency: faker.Random.Int(1, 10)));

        categoryIds.Add(created.RequireId());
    }

    // ------------------------------------------------------------------
    // 4. Statuses. The color column is UNIQUE, so track generated colors.
    // ------------------------------------------------------------------
    Console.WriteLine($"Creating {recordCount} statuses...");
    var statusIds = new List<int>();
    var usedStatusColors = new HashSet<int>();

    for (var i = 0; i < recordCount; i++)
    {
        int color;
        do
        {
            color = faker.Random.Int(0, 0xFFFFFF);
        }
        while (!usedStatusColors.Add(color));

        bool ready = faker.Random.Int(0, 3) == 0;
        CreateStatusDto new_status = new(
                    Name: Truncate($"{faker.Hacker.Adjective()} {faker.Hacker.Noun()}", 24),
                    Color: color,
                    Ready: ready,
                    Pending: ready ? false: faker.Random.Int(0, 3) == 0);
        var created = await api.PostAsync<EntityId>("/status", new_status);

        statusIds.Add(created.RequireId());
    }

    // ------------------------------------------------------------------
    // 5. Projects. Creating a project automatically adds its manager as a
    //    member, so we only add the extra members here.
    // ------------------------------------------------------------------
    Console.WriteLine($"Creating {recordCount} projects...");
    var projects = new List<ProjectSeed>();
    var now = DateTime.UtcNow;

    for (var i = 0; i < recordCount; i++)
    {
        var manager = faker.PickRandom(users);
        var startTime = now
            .AddDays(faker.Random.Int(-30, 30))
            .AddHours(faker.Random.Int(0, 23));
        var deadline = startTime.AddDays(faker.Random.Int(10, 365));

        var created = await api.PostAsync<EntityId>("/project", new CreateProjectDto(
            ManagerId: manager.Id,
            Name: Truncate(faker.Commerce.ProductName(), 64),
            Description: Truncate(faker.Lorem.Sentence(12), 512),
            StartTime: startTime,
            Deadline: deadline));

        projects.Add(new ProjectSeed(created.RequireId(), manager.Id));
    }

    Console.WriteLine("Adding project members...");
    var extraMemberCount = 0;
    foreach (var project in projects)
    {
        var candidates = users
            .Where(user => user.Id != project.ManagerId)
            .ToList();

        var extras = candidates
            .OrderBy(_ => faker.Random.Guid())
            .Take(2)
            .ToList();

        for (var i = 0; i < extras.Count; i++)
        {
            var role = i == 0 ? ProjectRole.Worker : ProjectRole.Observer;
            await api.PostEmptyAsync(
                $"/project/{project.Id}/member",
                new AddMemberDto(MemberId: extras[i].Id, Role: role));

            extraMemberCount++;
        }
    }

    // ------------------------------------------------------------------
    // 6. Tasks.
    // ------------------------------------------------------------------
    Console.WriteLine($"Creating {recordCount} tasks...");
    var taskIds = new List<int>();
    for (var i = 0; i < recordCount; i++)
    {
        var created = await api.PostAsync<EntityId>("/task", new CreateTaskDto(
            ProjectId: faker.PickRandom(projects).Id,
            Name: Truncate(faker.Commerce.ProductName(), 64),
            Description: Truncate(faker.Lorem.Sentences(2), 1024),
            Deadline: now.AddDays(faker.Random.Int(1, 120)),
            StatusId: faker.PickRandom(statusIds),
            AssigneeId: faker.PickRandom(users).Id,
            EstimatedTime: faker.Random.Int(30, 600)));

        taskIds.Add(created.RequireId());
    }

    // ------------------------------------------------------------------
    // 7. Task <-> Category associations (2-3 unique categories per task).
    // ------------------------------------------------------------------
    Console.WriteLine("Associating tasks with categories...");
    var taskCategoryCount = 0;
    foreach (var taskId in taskIds)
    {
        var selectedCategories = categoryIds
            .OrderBy(_ => faker.Random.Guid())
            .Take(faker.Random.Int(2, 3))
            .ToArray();

        await api.PostEmptyAsync(
            $"/task/{taskId}/categories/add",
            selectedCategories);

        taskCategoryCount += selectedCategories.Length;
    }

    // ------------------------------------------------------------------
    // 8. Task dependencies (one dependency per task).
    // ------------------------------------------------------------------
    Console.WriteLine("Creating task dependencies...");
    var dependencyCount = 0;
    if (taskIds.Count > 1)
    {
        for (var i = 0; i < taskIds.Count; i++)
        {
            var dependentId = taskIds[i];
            var dependencyId = taskIds[(i + 1) % taskIds.Count];

            await api.PostEmptyAsync($"/task/{dependentId}/dep/{dependencyId}");
            dependencyCount++;
        }
    }

    // ------------------------------------------------------------------
    // 9. Discussion posts, some of them replies to earlier posts.
    // ------------------------------------------------------------------
    Console.WriteLine($"Creating {recordCount} discussion posts...");
    var postIds = new List<int>();
    for (var i = 0; i < recordCount; i++)
    {
        int? parentId = null;
        if (postIds.Count >= 5 && faker.Random.Int(0, 99) < 35)
        {
            parentId = faker.PickRandom(postIds);
        }

        var created = await api.PostAsync<EntityId>("/discussion", new CreateDiscussionDto(
            TaskId: faker.PickRandom(taskIds),
            AuthorId: faker.PickRandom(users).Id,
            Title: Truncate(faker.Lorem.Sentence(4), 48),
            Body: Truncate(faker.Lorem.Sentences(2), 512),
            ParentId: parentId));

        postIds.Add(created.RequireId());
    }

    Console.WriteLine();
    Console.WriteLine($"Database populated with at least {recordCount} records per seeded table:");
    Console.WriteLine($"  users:               {users.Count}");
    Console.WriteLine($"  sessions:            {users.Count}");
    Console.WriteLine($"  categories:          {categoryIds.Count}");
    Console.WriteLine($"  statuses:            {statusIds.Count}");
    Console.WriteLine($"  projects:            {projects.Count}");
    Console.WriteLine($"  project_members:     {projects.Count + extraMemberCount}");
    Console.WriteLine($"  tasks:               {taskIds.Count}");
    Console.WriteLine($"  task_categories:     {taskCategoryCount}");
    Console.WriteLine($"  tasks_dependencies:  {dependencyCount}");
    Console.WriteLine($"  discussion_posts:    {postIds.Count}");
    Console.WriteLine();
    Console.WriteLine("Skipped (no HTTP API): roles, permissions, role_permissions,");
    Console.WriteLine("                       user_roles, audit_log.");

    return 0;
}
catch (Exception ex)
{
    Console.Error.WriteLine();
    Console.Error.WriteLine($"Population failed: {ex.Message}");
    return 1;
}

static void PrintUsage()
{
    Console.WriteLine("""
        Usage: dotnet run populate.cs -- [options]

        Options:
          --base-url <url>  Base URL of the API (default: http://localhost:44761)
          --count <n>       Records per main table (default: 60)
          --no-reset        Do not call POST /db_reset before seeding
          -h, --help        Show this help
        """);
}

static string CreateUsername(
    Faker faker,
    string firstName,
    int index,
    HashSet<string> usedUsernames)
{
    var baseName = AsciiAlphanumeric(firstName).ToLowerInvariant();
    while (baseName.Length < 2)
    {
        baseName = AsciiAlphanumeric(firstName).ToLowerInvariant();
    }

    if (baseName.Length > 20)
    {
        baseName = baseName[..20];
    }

    var candidate = $"{baseName}{index}";
    while (!usedUsernames.Add(candidate))
    {
        candidate = $"{baseName}{index}{faker.Random.AlphaNumeric(4).ToLowerInvariant()}";
    }

    return candidate;
}

static string AsciiAlphanumeric(string value)
{
    return new string(value
        .Where(c =>
            (c >= 'a' && c <= 'z') ||
            (c >= 'A' && c <= 'Z') ||
            (c >= '0' && c <= '9'))
        .ToArray());
}

static string Truncate(string value, int maxLength)
{
    if (value.Length <= maxLength)
    {
        return value;
    }

    return value[..maxLength].TrimEnd();
}

// ---------------------------------------------------------------------------
// Minimal HTTP client for the OpenAPI operations used by this script.
// ---------------------------------------------------------------------------
public sealed class ApiClient : IDisposable
{
    private static readonly JsonSerializerOptions JsonOptions = CreateJsonOptions();

    private readonly HttpClient _http = new()
    {
        Timeout = TimeSpan.FromSeconds(120),
    };

    private readonly Uri _baseUri;

    public ApiClient(string baseUrl)
    {
        _baseUri = new Uri(baseUrl.TrimEnd('/') + "/", UriKind.Absolute);
    }

    public async Task<T> GetAsync<T>(string path)
    {
        using var response = await SendAsync(HttpMethod.Get, path, body: null);
        return await ReadAsync<T>(response, $"GET {path}");
    }

    public async Task<T> PostAsync<T>(string path, object body)
    {
        using var response = await SendAsync(HttpMethod.Post, path, body);
        return await ReadAsync<T>(response, $"POST {path}");
    }

    public async Task PostEmptyAsync(string path, object? body = null)
    {
        using var response = await SendAsync(HttpMethod.Post, path, body);
    }

    public void Dispose() => _http.Dispose();

    private async Task<HttpResponseMessage> SendAsync(
        HttpMethod method,
        string path,
        object? body)
    {
        using var request = new HttpRequestMessage(
            method,
            new Uri(_baseUri, path.TrimStart('/')));

        if (body is not null)
        {
            // Use the runtime type so records serialize with all their properties.
            var json = JsonSerializer.Serialize(body, body.GetType(), JsonOptions);
            request.Content = new StringContent(json, Encoding.UTF8, "application/json");
        }

        var response = await _http.SendAsync(request);
        if (response.IsSuccessStatusCode)
        {
            return response;
        }

        var status = response.StatusCode;
        var reason = response.ReasonPhrase;
        var details = await response.Content.ReadAsStringAsync();
        response.Dispose();

        throw new HttpRequestException(
            $"{method} {path} -> {(int)status} {reason}: {DescribeError(details)}");
    }

    private static async Task<T> ReadAsync<T>(
        HttpResponseMessage response,
        string operation)
    {
        var json = await response.Content.ReadAsStringAsync();
        if (string.IsNullOrWhiteSpace(json))
        {
            throw new InvalidOperationException(
                $"{operation} returned an empty response.");
        }

        return JsonSerializer.Deserialize<T>(json, JsonOptions)
            ?? throw new InvalidOperationException(
                $"{operation} returned an invalid JSON response.");
    }

    private static string DescribeError(string body)
    {
        if (string.IsNullOrWhiteSpace(body))
        {
            return "(empty response body)";
        }

        try
        {
            var error = JsonSerializer.Deserialize<ApiError>(body, JsonOptions);
            if (error is not null)
            {
                return $"{error.Details} (HTTP {error.Code})";
            }
        }
        catch (JsonException)
        {
            // Not an HttpError payload; fall through and show the raw body.
        }

        return body;
    }

    private static JsonSerializerOptions CreateJsonOptions()
    {
        var options = new JsonSerializerOptions(JsonSerializerDefaults.Web)
        {
            PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        };

        options.Converters.Add(
            new JsonStringEnumConverter<ProjectRole>(JsonNamingPolicy.SnakeCaseLower));

        return options;
    }
}

// ---------------------------------------------------------------------------
// Request models (see components/schemas in the OpenAPI document).
// ---------------------------------------------------------------------------
public sealed record SignupDto(
    string Email,
    string Username,
    string Password,
    string FirstName,
    string SecondName,
    string? Bio = null);

public sealed record SigninDto(
    string Identity,
    string Password);

public sealed record CreateCategoryDto(
    string Name,
    int BadgeColor,
    int Urgency);

public sealed record CreateStatusDto(
    string Name,
    int Color,
    bool Ready,
    bool Pending);

public sealed record CreateProjectDto(
    int ManagerId,
    string Name,
    string Description,
    DateTime StartTime,
    DateTime Deadline);

public sealed record AddMemberDto(
    int MemberId,
    ProjectRole Role);

public sealed record CreateTaskDto(
    int ProjectId,
    string Name,
    string? Description,
    DateTime Deadline,
    int StatusId,
    int AssigneeId,
    int EstimatedTime);

public sealed record CreateDiscussionDto(
    int TaskId,
    int AuthorId,
    string Title,
    string Body,
    int? ParentId = null);

public enum ProjectRole
{
    Observer,
    Manager,
    Worker,
}

// ---------------------------------------------------------------------------
// Minimal response models. Extra properties in the API response are ignored.
// ---------------------------------------------------------------------------
public sealed record HealthResponse(string Status);

public sealed record SessionResponse(string AccessToken);

public sealed record EntityId(int? Id)
{
    public int RequireId() =>
        Id ?? throw new InvalidOperationException(
            "The API returned an entity without an id.");
}

public sealed record ApiError(int Code, string Details);

// ---------------------------------------------------------------------------
// Seed bookkeeping types (never serialized).
// ---------------------------------------------------------------------------
public sealed record UserSeed(int Id, string Username);

public sealed record ProjectSeed(int Id, int ManagerId);
