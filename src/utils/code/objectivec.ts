/** Objective-C（NSURLSession）代码生成 */

import { esc, Req } from "./shared";
export function genObjectiveC(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），请使用 NSURLSessionUploadTask 配合 multipart body 构造请求");
  }
  out.push("#import <Foundation/Foundation.h>");
  out.push("");
  out.push("int main(int argc, const char * argv[]) {");
  out.push("    @autoreleasepool {");
  out.push(`        NSURL *url = [NSURL URLWithString:@"${esc(r.url, '"')}"];`);
  out.push("        NSMutableURLRequest *request = [NSMutableURLRequest requestWithURL:url];");
  out.push(`        request.HTTPMethod = @"${r.method}";`);
  for (const h of r.headers) {
    out.push(`        [request setValue:@"${esc(h.value, '"')}" forHTTPHeaderField:@"${esc(h.key, '"')}"];`);
  }
  if (r.body) {
    out.push(`        request.HTTPBody = [@"${esc(r.body, '"')}" dataUsingEncoding:NSUTF8StringEncoding];`);
  }
  out.push("");
  out.push("        dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);");
  out.push("        NSURLSession *session = [NSURLSession sharedSession];");
  out.push("        NSURLSessionDataTask *task = [session dataTaskWithRequest:request");
  out.push("            completionHandler:^(NSData *data, NSURLResponse *response, NSError *error) {");
  out.push("                if (error) {");
  out.push("                    NSLog(@\"Error: %@\", error);");
  out.push("                } else if (data) {");
  out.push("                    NSLog(@\"%@\", [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding]);");
  out.push("                }");
  out.push("                dispatch_semaphore_signal(semaphore);");
  out.push("            }];");
  out.push("        [task resume];");
  out.push("        dispatch_semaphore_wait(semaphore, DISPATCH_TIME_FOREVER);");
  out.push("    }");
  out.push("    return 0;");
  out.push("}");
  return out.join("\n");
}
